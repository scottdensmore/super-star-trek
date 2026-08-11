#include "sst.h"
#include "tui.h"
#include <curses.h>
#include <term.h>

/* Curses backend for the full-screen interface (sst -t).
 *
 * Layout: a quadrant panel (top left) and status panel (top right)
 * that repaint from the live game state before every prompt, and a
 * scrolling message window below them that carries the game's normal
 * conversation. All game output reaches this file through proutn()
 * and friends; panel text comes from the formatters in tuifmt.c.
 */

#define PANELH (13)		/* border + header + 10 grid rows + border */
#define QUADW (29)		/* border + 25-char grid line + padding */

int tui_active;		/* declared in tui.h; see the note there */
int tui_ingame;

static WINDOW *wquad, *wstat, *wmsg;
static int use_colour;

/* One pair per foreground; the background stays whatever the terminal
 * already uses, so the panels sit in the player's own colour scheme.
 *
 * Nothing is carried by colour alone. Every cell is a distinct glyph
 * and every coloured status value also spells its state out in words,
 * so a player who cannot tell red from green reads exactly what a
 * monochrome terminal shows. Keep it that way. */
#define CP_HOSTILE 1
#define CP_BASE    2
#define CP_STAR    3
#define CP_PLANET  4
#define CP_THING   5
#define CP_GOOD    6
#define CP_WARN    7

static void start_colour(void) {
	short bg = -1;
	const char *nc = getenv("NO_COLOR");

	/* NO_COLOR is how a player asks for plain text without giving up
	   the full-screen layout, which TERM=dumb would also cost them. */
	use_colour = has_colors() && !(nc != NULL && *nc != '\0');
	if (!use_colour) return;	/* the layout works without it */
	start_color();
	if (use_default_colors() == ERR)
		bg = COLOR_BLACK;
	init_pair(CP_HOSTILE, COLOR_RED, bg);
	init_pair(CP_BASE, COLOR_CYAN, bg);
	init_pair(CP_STAR, COLOR_BLUE, bg);
	init_pair(CP_PLANET, COLOR_GREEN, bg);
	init_pair(CP_THING, COLOR_MAGENTA, bg);
	init_pair(CP_GOOD, COLOR_GREEN, bg);
	init_pair(CP_WARN, COLOR_YELLOW, bg);
}

static chtype attr_for_cell(char c) {
	if (!use_colour) return A_NORMAL;
	switch (cell_class(c)) {
		case CELL_HOSTILE:	return COLOR_PAIR(CP_HOSTILE) | A_BOLD;
		/* The ship takes no colour pair on purpose: bold alone keeps
		   the terminal's own foreground, so it stands out on a light
		   background as surely as on a dark one. A hardcoded white
		   would be invisible on the former. */
		case CELL_SHIP:		return A_BOLD;
		case CELL_BASE:		return COLOR_PAIR(CP_BASE) | A_BOLD;
		/* Stars are the most numerous thing on the grid and the least
		   worth looking at, so they stay quiet -- and yellow is spoken
		   for by the status panel, where it means caution. */
		case CELL_STAR:		return COLOR_PAIR(CP_STAR);
		case CELL_PLANET:	return COLOR_PAIR(CP_PLANET);
		case CELL_WEB:		return COLOR_PAIR(CP_THING);
		case CELL_THING:	return COLOR_PAIR(CP_THING) | A_BOLD;
		case CELL_HIDDEN:	return A_DIM;
		default:		return A_NORMAL;
	}
}

static chtype attr_for_status(int line) {
	if (!use_colour) return A_NORMAL;
	switch (status_class(line)) {
		case STAT_GOOD:		return COLOR_PAIR(CP_GOOD);
		case STAT_WARN:		return COLOR_PAIR(CP_WARN);
		case STAT_BAD:		return COLOR_PAIR(CP_HOSTILE) | A_BOLD;
		case STAT_DOCKED:	return COLOR_PAIR(CP_BASE) | A_BOLD;
		default:		return A_NORMAL;
	}
}

/* How much of a panel line there is room for: the window less its right
 * border, less the two columns the text is indented by -- the left
 * border and the padding column beside it.
 *
 * Without it the panels do not clip, whatever make_windows() says they
 * do. waddch past the last column wraps to the next row of the same
 * window, so a line too long for a terminal shrunk below the 72x24
 * minimum landed on the row below -- over its left border and padding,
 * and for the last line over the bottom border, which is where
 * `xtsKlingons Left 2` and a bottom border reading `     7.00qj` came
 * from. At the minimum and above every line fits and nothing here
 * changes -- but only just: `Shields       DAMAGED, 100% 2500.0 units`
 * is 40 columns against the 40 the status panel has at 72, an exact fit
 * with nothing to spare. One more word on any status line and it would
 * be the supported minimum that clips. Nothing checks that; see #89. */
static int panel_room(WINDOW *w) {
	return getmaxx(w) - 3;
}

/* Whether a panel line has a row to be drawn on: inside the box, not on
 * either border. The same question as panel_room() on the other axis,
 * and it went unasked -- the draw loops took the row from the caller,
 * which always asks for all eleven however short the terminal is.
 *
 * A window shrunk to twelve rows still has row eleven, so the tenth grid
 * line was drawn over the bottom border. At eleven rows it has no row
 * eleven at all: wmove() fails, leaves the cursor where the previous
 * line left it, and the whole line lands on the row above --
 * `mq 9  . . . . . . . . . .10 .x`, two grid rows and the border
 * sharing one line. */
static int panel_has_row(WINDOW *w, int row) {
	return row >= 1 && row < getmaxy(w) - 1;
}

/* Clipping a status line in silence is worse than the wrapping it
 * replaces, because what is left reads as a smaller, perfectly
 * plausible reading of the same field: `Torpedoes     1` for ten of
 * them, `Energy        367` for 3675.25, `Shields       DOWN, 10` for
 * 100%. Those are the fields a player checks in a fight, on a panel
 * that now looks intact and so gets believed. Garbage announced
 * itself; a truncated number does not.
 *
 * So the last column a clipped line has goes to a marker instead of to
 * one more digit. `Energy        36>` cannot be misread as a quantity,
 * and it says which lines to widen the terminal to see. */
static char clip_at(const char *s, int i, int room) {
	return (i == room - 1 && s[i + 1] != '\0') ? '>' : s[i];
}

/* Draw a grid line a cell at a time so each symbol carries its own
 * colour. Row labels and spacing classify as plain and stay uncoloured. */
static void draw_quad_line(int row, const char *s) {
	int i, room = panel_room(wquad);

	if (!panel_has_row(wquad, row)) return;
	wmove(wquad, row, 2);
	for (i = 0; s[i] != '\0' && i < room; i++) {
		char c = clip_at(s, i, room);
		chtype a = attr_for_cell(c);
		if (a != A_NORMAL) wattron(wquad, a);
		waddch(wquad, (unsigned char)c);
		if (a != A_NORMAL) wattroff(wquad, a);
	}
}

/* The label stays plain and the value takes the colour, so a glance
 * down the panel picks out the fields that are shouting. */
static void draw_status_line(int row, int line, const char *s) {
	chtype a = attr_for_status(line);
	int i, room = panel_room(wstat);

	if (!panel_has_row(wstat, row)) return;
	wmove(wstat, row, 2);
	for (i = 0; s[i] != '\0' && i < room; i++) {
		if (i == STATLABEL && a != A_NORMAL) wattron(wstat, a);
		waddch(wstat, (unsigned char)clip_at(s, i, room));
	}
	if (a != A_NORMAL) wattroff(wstat, a);
}

/* The size the windows were last built for, so a read that comes back
 * empty can be told apart from one interrupted by the terminal
 * changing shape. */
static int builtlines, builtcols;

/* Lay the three windows out for the terminal as it is now. Called
 * again after a resize, so it deletes what was there first.
 *
 * A terminal that has shrunk below the 72x24 the display asks for at
 * startup is not torn down mid-game: the panels keep their size and
 * the screen clips, which is ugly but leaves the player where they
 * were. Ending curses under them to say the window is too small would
 * be a worse answer to a mouse drag. */
static void make_windows(void) {
	int msgh, statw, msgw, panelh, quadw;

	builtlines = LINES;
	builtcols = COLS;
	/* On a terminal too small for them the panels are as big as there
	   is room for, not as big as they would like. Asking for the full
	   size on a smaller screen is not merely optimistic: the window
	   then reports room the player cannot see, and the guards that
	   decide whether a title or a status line fits believe it.

	   The status panel used to be put back to its full depth on every
	   resize, which left it taller than the screen, its bottom border
	   off the bottom, and its last line drawn where that border should
	   have been. */
	panelh = LINES < PANELH ? LINES : PANELH;
	quadw = COLS < QUADW ? COLS : QUADW;
	msgh = LINES-PANELH > 1 ? LINES-PANELH : 1;
	statw = COLS-QUADW > 1 ? COLS-QUADW : 1;
	msgw = COLS-2 > 1 ? COLS-2 : 1;
	if (wmsg == NULL) {
		wquad = newwin(PANELH, QUADW, 0, 0);
		wstat = newwin(PANELH, statw, 0, QUADW);
		wmsg = newwin(msgh, msgw, PANELH, 1);
	} else {
		/* Resized rather than remade, because the message window
		   holds the conversation and a fresh one would be empty --
		   including the prompt the player is being asked to answer
		   at that very moment, which would leave the game looking
		   hung. The panels are redrawn from the game state either
		   way. */
		/* The quadrant panel is resized here too, though nothing
		   about the layout asks it to change size. curses resizes a
		   window that spans the screen along with the screen, down
		   and back up again, so a panel squeezed past its own size
		   stops being 29 columns and starts tracking the terminal:
		   it came back as wide as the whole screen after a narrow
		   squeeze, and as tall as it after a short one, which left
		   the message window off the bottom with the prompt on it.
		   The fault outlived the squeeze, at a size where nothing on
		   screen explained it. */
		wresize(wquad, panelh, quadw);
		wresize(wstat, panelh, statw);
		wresize(wmsg, msgh, msgw);
		/* And the message window is moved back, not only resized. A
		   squeeze leaving no room under the panels puts its origin
		   at or past the bottom of the screen, and resize_term()
		   then treats it as living below the screen and shifts it
		   down again by however much the terminal later grew: the
		   panels came back whole while the conversation stayed gone,
		   prompt and all, on a screen with eleven empty rows waiting
		   for it.

		   After the resizes, not before. mvwin() refuses a move that
		   would not fit at the destination *at the window's current
		   size*, so moving first asks to put a window still as wide
		   as the squeeze was into a narrower terminal. Squeeze to 82
		   columns and come back to 80 and the move is refused, the
		   window stays off-screen, and nothing tries again -- the
		   display that came back looked right and swallowed
		   everything typed into it.

		   The return is dropped on purpose: ERR comes back only
		   where there is nowhere legal to put the window -- LINES
		   at or under PANELH, or fewer than two columns -- and at
		   those sizes the panels are the whole screen and nothing
		   is drawn below them anyway. The next resize asks again. */
		mvwin(wmsg, PANELH, 1);
	}
	/* Set every time, not only on the first: keypad in particular is
	   what turns a resize into KEY_RESIZE rather than into whatever
	   bytes the terminal sent, and losing it would let the next drag
	   answer a paging prompt. */
	scrollok(wmsg, TRUE);
	keypad(wmsg, TRUE);
}

/* Whether the terminal changed shape since the windows were built.
 * Asked of the terminal rather than of LINES and COLS: those are only
 * brought up to date when curses processes the resize, and a read that
 * came back empty needs the answer before that has happened. */
static int resized(void) {
	return is_term_resized(builtlines, builtcols);
}

/* The line the game is part way through writing -- everything since the
 * last newline. It is kept because a resize can take it off the screen
 * and there is no getting it back from curses.
 *
 * Curses clips the message window when it processes the resize, which
 * it does before handing the game a KEY_RESIZE: by the time the game
 * knows the terminal moved, the bottom rows are already gone. When the
 * terminal is now shorter, those rows are where the newest text is --
 * and the newest text is the prompt the player is being asked to answer
 * or the pause waiting for a keystroke. Losing it leaves a screen that
 * is waiting for something without saying so.
 *
 * A whole transcript would let the conversation be reflowed; this is the
 * one line that matters, and it is what lets the prompt be put back. */
static char curline[256];
static int curlen;

/* The last line the game finished, kept for the prompts that end their
 * own: prout() appends the newline, so by the time such a prompt stops
 * to wait, curline is empty and there is nothing to put back. Those
 * questions vanished on a resize while the reader went on waiting, and
 * the next keystroke was eaten by a prompt that was not on screen.
 * Issue #112.
 *
 * Only lines with something on them are kept. pause() wipes its own
 * prompt with "\r ... \r", so a run of blanks arrives here as a
 * finished line, and keeping it would put an empty row back in place of
 * a question. */
static char lastline[sizeof curline];
static int lastlen;

/* Whether a reader is waiting for the player right now. lastline is
 * only worth putting back then: at a prompt it is the prompt, and
 * anywhere else it is an old line that may have scrolled away for good
 * reason. tui_readline() and tui_getch() are the two that wait;
 * tui_gameover() also repaints, and has no prompt pending. */
static int reader_waiting;

static void end_line(void) {
	int i;

	for (i = 0; i < curlen; i++) {
		if (curline[i] != ' ') {
			memcpy(lastline, curline, (size_t)curlen);
			lastline[curlen] = '\0';
			lastlen = curlen;
			break;
		}
	}
	curlen = 0;
}

static void note_out(const char *s) {
	for (; *s != '\0'; s++) {
		/* The two end curline alike and are kept apart on
		   purpose. A newline is the game finishing a line, so
		   end_line() records it. A carriage return is pause()
		   wiping its own prompt with "\r ... \r" -- and at the
		   first of those curline still holds [HIT SPACE BAR TO
		   CONTINUE], so recording there put the pager's prompt
		   back over a question the game was really waiting on.
		   Folding them together again reintroduces exactly that.
		   The blanks between the two \r were always dropped; the
		   prompt before the first one was the part that was not. */
		if (*s == '\n')
			end_line();
		else if (*s == '\r')
			curlen = 0;	/* see end_line: a wipe, not a line */
		else if (curlen < (int)sizeof curline - 1)
			curline[curlen++] = *s;
	}
	curline[curlen] = '\0';
}

/* What tui_readline has echoed of the answer so far, while it is
 * waiting; NULL at a pause, which has no answer to echo. The restore
 * below needs it because the bottom of the conversation is the game's
 * unfinished line *and* the player's unfinished answer, and it has to
 * recognise the pair to tell "still on screen" from "clipped away". */
static const char *pending_answer;

/* Whether the repaint left the cursor at the end of the game's line,
 * with the answer to be written after it. The repaint runs whichever
 * way the terminal moved, so what a repaint leaves this false for is
 * having no part-written line to move the cursor to: a prompt that
 * ended its own line and was not kept -- see lastline, which covers
 * most of those now, and #117 for the case it does not. It is also
    false where no
 * repaint happened -- a KEY_RESIZE carrying no actual size change,
 * which sync_size() returns from before touching anything. */
static int cursor_at_prompt;

/* Whether a window row holds nothing but blanks. */
static int row_blank(int r) {
	char row[256];
	int i;

	row[0] = '\0';
	mvwinnstr(wmsg, r, 0, row, (int)sizeof row - 1);
	for (i = 0; row[i] != '\0'; i++)
		if (row[i] != ' ') return FALSE;
	return TRUE;
}

/* Put the unfinished line back on screen if the resize took it away,
 * and leave the cursor at the end of it either way.
 *
 * Whether it is still there is answered by looking for it, because the
 * cursor is no guide: until the conversation has filled the window
 * curses leaves it a row off, which read as "the prompt is gone" and
 * appended a second copy of every setup question.
 *
 * Looking is not enough on its own, though. " COMMAND> " is the most
 * repeated string in the game, so a search that takes any match finds
 * an answered prompt from earlier in the conversation, decides nothing
 * was lost, and parks the cursor in the middle of the history -- the
 * player then types into an old command and the game reads a line the
 * screen never showed. A match counts only as the last thing on the
 * screen: nothing but blanks after it on its row, and nothing but blank
 * rows below.
 *
 * Missing, and it gets a row of its own. The window scrolls by one
 * first, because writing a newline would clear to the end of the row
 * the clip left the cursor on, taking the tail of the line above with
 * it -- a sentence cut off mid-word. Losing the oldest row is what this
 * window does all the time; mangling the newest is not.
 *
 * A terminal that loses columns is the awkward case: it does not remove
 * the line, it mangles it. Curses truncates each row rather than
 * reflowing it, so a pair that no longer fits stops being the string
 * this searches for -- and that same absence of reflow is what makes
 * the stump's height, computed below, exact. */
static void restore_curline(int oldmsgw) {
	/* want holds curline and the answer: 160 covers readinput()'s
	   callers, which pass 128-byte buffers (line[] in sst.c,
	   winner[] in finish.c) of which the reader fills at most 126.
	   onscreen and row_blank's row are smaller than a very wide
	   terminal on purpose -- nothing the game writes reaches column
	   255, the longest line being a prompt plus that 126-character
	   answer -- so a row with content past there cannot exist. Raise
	   the input buffer and these want revisiting. */
	char onscreen[512], want[sizeof curline + 160], joined[2048];
	const char *line = curline;
	char *found;
	int r, last, maxy, width, i, tail_blank, wantlen;
	int linelen = curlen, ended = FALSE;

	/* Nothing part written means either that there is nothing to put
	   back, or that the prompt ended its own line -- prout() appends
	   the newline, so curline is already empty by the time such a
	   question stops to wait. lastline is that line, and it is only
	   worth putting back while a reader is waiting, since anywhere
	   else it is old conversation that scrolled away for a reason. */
	if (curlen == 0) {
		if (!reader_waiting || lastlen == 0) return;
		/* Only before the player has typed anything. Once there is
		   an answer the two sit on separate rows with curses'
		   padding between them, and every attempt at spanning that
		   went wrong in a different way: dropping the answer from
		   the comparison reprinted the question once per drag step,
		   and padding it back turned a one-row height shrink into a
		   second copy. What is left unfixed is milder than either
		   -- what is left is exactly what the game did before this
		   change, which at some shapes is a blank message window
		   with the reader still waiting. Worse than restoring it
		   would be; better than restoring it twice. Issue #117. */
		if (pending_answer != NULL && *pending_answer != '\0')
			return;
		line = lastline;
		linelen = lastlen;
		ended = TRUE;
	}
	/* What the bottom of the conversation should read: the line the
	   game is part way through, and after it whatever the reader has
	   echoed of the answer. Comparing against the prompt alone made
	   every pending answer look like something drawn over the line,
	   so an ordinary drag reprinted the prompt at each step and stood
	   eight copies of it up the window. */
	maxy = getmaxy(wmsg);
	width = getmaxx(wmsg);
	/* An ended line puts the answer on the row below, and curses pads
	   the rest of the prompt's row with blanks to get there. Those
	   blanks are in the joined rows, so they have to be in `want`
	   too: without them the answer never matched, the tail test
	   failed, and a drag reprinted the question at every step --
	   the same copies-up-the-window this comparison exists to stop,
	   reintroduced for the ended case. */
	i = ended && width > 0 && linelen % width != 0
	    ? width - linelen % width : 0;
	snprintf(want, sizeof want, "%s%*s%s", line, i, "",
		 pending_answer == NULL ? "" : pending_answer);
	wantlen = (int)strlen(want);
	for (r = maxy - 1; r >= 0; r--) {
		if (!row_blank(r)) break;
	}
	last = r;
	/* Read back as many rows as the line takes when it wraps, not
	   one: a question and an answer longer than the window is two
	   rows on screen and a single string here, so a one-row search
	   could never match it and reprinted at every resize -- four
	   copies up the window over a slow drag. Curses pads each row to
	   the full width, so joining them rebuilds the text exactly. */
	if (r >= 0 && width > 0) {
		int start = r - wantlen / width;
		int rr;

		if (start < 0) start = 0;
		joined[0] = '\0';
		for (rr = start; rr <= r; rr++) {
			onscreen[0] = '\0';
			mvwinnstr(wmsg, rr, 0, onscreen,
				  (int)sizeof onscreen - 1);
			strncat(joined, onscreen,
				sizeof joined - strlen(joined) - 1);
		}
		found = strstr(joined, want);
		if (found != NULL) {
			tail_blank = TRUE;
			for (i = (int)(found - joined) + wantlen;
			     joined[i] != '\0'; i++)
				if (joined[i] != ' ') tail_blank = FALSE;
			if (tail_blank) {
				/* At the end of the prompt, not of the
				   answer: the reader writes the answer
				   again from here, over the identical
				   characters already there. */
				i = (int)(found - joined) + linelen;
				/* A line that ended takes the answer on
				   the row below it, not after it -- and
				   that row can be off the bottom, where
				   the prompt is the window's last. wmove()
				   fails there and leaves the cursor where
				   the row scan parked it, at column 0 of
				   the question, so the echo typed the
				   answer over it: ` yyy you sure?`. */
				if (ended) i = (i / width + 1) * width;
				r = start + i / width;
				if (r > maxy - 1 && maxy > 1) {
					wscrl(wmsg, 1);
					r = maxy - 1;
					i = 0;
				} else if (r > maxy - 1) {
					/* One row, so nothing below and
					   nothing to scroll into: scrolling
					   would discard the prompt just
					   matched. Leave the cursor after
					   it, as the write path does. */
					r = maxy - 1;
					i = (int)(found - joined) + linelen;
				}
				/* Only where the cursor really moved: a
				   failed wmove leaves it wherever the row
				   scan left it, and claiming otherwise is
				   what puts the echo in the wrong column. */
				if (wmove(wmsg, r, i % width) != ERR) {
					cursor_at_prompt = TRUE;
					return;
				}
			}
		}
	}
	/* Where to put it. A terminal whose width changed left the stump on
	   screen, taking as many rows as the pair filled at the old
	   width -- which is arithmetic, not something to go looking for.
	   The old width is what the stump was laid out at whichever way
	   the terminal then moved, so this holds on a grow as well: it is
	   how many rows are already occupied, not how many the line needs
	   now.
	   Rub those rows out and write the line in their place; scrolling
	   instead is what left one stump per step of a narrowing drag,
	   six of them up the window with the conversation pushed off the
	   top.
	   A width that did not change is the other case, and the else
	   below is where it is argued. */
	if (oldmsgw > 0) {
		int total = linelen;
		int rows, rr;

		if (pending_answer != NULL)
			total += (int)strlen(pending_answer);
		/* Apart, for an ended line: the answer starts a row of its
		   own, so the two do not share one. Added together they
		   under-count and the erase leaves a row of the stump. */
		/* Defensive, not live: the #117 gate means pending_answer
		   here is NULL or empty, and with an empty one the rounding
		   leaves rows unchanged. It is right for the day #117 is
		   fixed and an answer really does start its own row.
		   Guarded like the padding above, and for the same reason:
		   a prompt that exactly fills its row needs no filler, and
		   adding one anyway made rows one too high, so the erase
		   would take a row of live conversation with it. */
		if (ended && pending_answer != NULL && linelen % oldmsgw != 0)
			total += oldmsgw - linelen % oldmsgw;
		/* One row per oldmsgw columns, and a line that fills its
		   last row exactly needs no extra: total/oldmsgw+1 says
		   two for a line of exactly one row's width, and with the
		   anchoring below that would rub out a row of live
		   conversation above the stump. */
		rows = total == 0 ? 1 : (total - 1) / oldmsgw + 1;
		/* Anchored to the last row with anything on it, not to the
		   bottom of the window. They are the same once the
		   conversation has filled it, and different for the whole
		   setup conversation and after every clearscreen() -- there
		   the live line sits mid-window, and writing at the bottom
		   left the stump above it with blank rows in between. */
		r = (last >= 0 ? last : maxy - 1) - rows + 1;
		if (r < 0) r = 0;
		for (rr = r; rr < maxy; rr++) {
			wmove(wmsg, rr, 0);
			wclrtoeol(wmsg);
		}
		wmove(wmsg, r, 0);
	} else {
		/* A width that did not change is the other case, and it
		   comes both ways. Lost rows: the line is gone altogether
		   and the bottom rows hold older conversation worth
		   keeping, so this scrolls by one and writes below it.
		   Gained rows: the line is on screen as a stump, and the
		   scroll is what carries it off the top.
		   Bottom-anchored, unlike the branch above, and on a grow
		   that shows: a height-only squeeze and return leaves the
		   pair on the last two rows with blank ones above it. That
		   is reproducible -- 72x24, a wrapping answer, 72x14, back
		   to 72x24 -- and it is this change that made it visible,
		   by restoring on a grow at all. Left alone deliberately.
		   The scroll is not only making room: it is what carries
		   the stump off the top of the window. Anchoring under the
		   conversation without it leaves the stump on screen above
		   the line, which is worse than a gap.
		   Erasing instead, the way the branch above does, is not
		   the arithmetic problem it looks like -- the width did not
		   change, so getmaxx(wmsg) already is the old width and the
		   row count is one line away. What stops it is that this
		   branch serves two opposite states. On a height grow the
		   stump is on screen and last names its last row, so
		   erasing back from there is right. On a height shrink
		   wresize() truncated the line away and last names live
		   conversation, so the same erase would wipe the line above
		   the prompt where the scroll preserves it. A fix has to
		   tell the two apart first, which is more than this is
		   worth. Issue #114. */
		wscrl(wmsg, 1);
		wmove(wmsg, maxy - 1, 0);
	}
	waddstr(wmsg, line);
	/* Same again: the answer to a question that ended its own line
	   belongs on the row after it -- unless there is no row after
	   it. A one-row message window scrolls the prompt away the
	   moment the newline lands, which is the blank screen #112 is
	   about, reached by the fix for it. */
	if (ended && maxy > 1) waddch(wmsg, '\n');
	cursor_at_prompt = TRUE;
}

/* Adopt a new terminal size if there is one. Called before every
 * repaint rather than only where a resize is reported, because the
 * report does not always come: a terminal dragged while the game is
 * blocked reading a line gives curses nothing to hand back, and the
 * next thing that happens is a repaint.
 *
 * The message window's contents are lost -- curses cannot reflow them
 * and the game keeps no transcript -- so the panels come back and the
 * conversation carries on from wherever it had got to. */
static void sync_size(void) {
	int oldcols = builtcols;

	if (!resized()) return;
	cursor_at_prompt = FALSE;
	resize_term(0, 0);	/* adopt whatever the terminal is now */
	make_windows();		/* which is what moves builtlines/builtcols on */
	touchwin(wmsg);		/* the panels redraw themselves; this does not */
	/* On a grow as well as a shrink. A shrink writes the line into
	   whatever room is left, which can be one row and narrower than
	   the line: it wraps, scrolls, and leaves the tail. Growing back
	   used to do nothing, so a corner drag returned to a stump --
	   ` , or frozen game?` of a question still waiting to be
	   answered, with the first keystroke of the answer eaten by the
	   reader still sitting behind it.
	   This was left out once because asking on a grow stood a second
	   copy of a question up the window whenever the answer being
	   typed was long enough to wrap. That was the one-row search:
	   restore_curline() joins as many rows as the pair takes before
	   looking, so an intact line is found and left alone whichever
	   way the terminal moved, and only a line that really is gone or
	   broken gets rewritten.
	   The old width is what says how many rows the stump occupies,
	   so it is passed whenever the width changed at all rather than
	   only when it shrank. */
	restore_curline(COLS != oldcols ? oldcols - 2 : 0);
	wnoutrefresh(wmsg);
}


/* Whether this terminal can actually run the full-screen interface.
 *
 * initscr() is no good for asking: given a TERM it doesn't know it
 * prints an error and exits the process, taking the game with it. Worse
 * is a terminal it does know but that cannot address the cursor -- TERM
 * =dumb, which is what Emacs' shell buffer sets. There curses starts
 * happily and then draws nothing usable, so the game looks hung.
 *
 * setupterm() reports both cases instead of exiting. The answer is
 * cached because main() asks again to word the fallback notice. */
int tui_terminal_capable(void) {
	static int checked = FALSE, capable = FALSE;
	char *cup;
	int err;

	if (checked) return capable;
	checked = TRUE;
	if (setupterm(NULL, fileno(stdout), &err) == ERR)
		return capable;		/* no such terminal type */
	cup = tigetstr("cup");
	capable = cup != NULL && cup != (char *)-1;
	return capable;
}

int tui_init(void) {
	/* Both checks come before initscr(): with no terminal, or one it
	   cannot drive, curses either kills the process outright or leaves
	   the player staring at a screen that never fills in. */
	if (!stdio_is_terminal())
		return FALSE;
	if (!tui_terminal_capable())
		return FALSE;
	initscr();
	if (LINES < 24 || COLS < 72) {
		endwin();
		return FALSE;
	}
	cbreak();
	noecho();
	start_colour();
	make_windows();
	tui_active = TRUE;
	atexit(tui_shutdown);
	tui_refresh_panels();
	return TRUE;
}

/* However a game ended, the panels are now describing one that is
 * over -- and after a destruction, a ship that no longer exists. Every
 * ending goes through here rather than clearing the flag itself, so a
 * fourth one cannot quietly forget to. */
void tui_gameover(void) {
	tui_ingame = FALSE;
	if (tui_active) tui_refresh_panels();	/* now, not at the next
						   prompt */
}

void tui_shutdown(void) {
	if (!tui_active) return;
	tui_active = FALSE;
	endwin();
}

void tui_refresh_panels(void) {
	char buf[FMTBUFLEN];
	int i;

	sync_size();
	werase(wquad);
	box(wquad, 0, 0);
	/* Four strings in these panels are written whole rather than a cell
	   at a time -- the two titles, the in-game one that replaces the
	   first, and the SENSORS DAMAGED caption below -- so the clipping
	   above does not cover any of them, and each needs room asked for
	   before it is drawn. One longer than its window wraps onto the row
	   beneath and sits on that row's left border, which is the very
	   fault the clipping exists to remove; the caption, being on the
	   window's last row, runs into the corner instead. " Status " does
	   it below 40 columns, the quadrant's in-game title below 19, the
	   bare " Quadrant " below 13 -- which stands in before a game, and
	   inside one wherever the coordinates no longer fit -- and the
	   caption below 20. Both windows shrink, and both report it:
	   make_windows() asks for each of them clamped to what the screen
	   can show, so panel_room() never offers room the player cannot
	   see. A box with no caption is the better of the two, and at these
	   widths the panel underneath is the only thing saying which is
	   which. */
	if (panel_room(wquad) >= (int)strlen(" Quadrant "))
		mvwaddstr(wquad, 0, 2, " Quadrant ");
	werase(wstat);
	box(wstat, 0, 0);
	if (panel_room(wstat) >= (int)strlen(" Status "))
		mvwaddstr(wstat, 0, 2, " Status ");
	/* Nothing to show before a game is set up or after one ends; the
	   formatters work the condition out for themselves, so nothing
	   here writes to the game state. */
	if (tui_ingame) {
		/* The guard measures the widest the title can be, which is
		   also the only width it is: quadrant coordinates are always
		   single digits, so " Quadrant 8 - 8 " is what gets drawn,
		   not an upper bound on it. */
		if (panel_room(wquad) >= (int)strlen(" Quadrant 8 - 8 "))
			mvwprintw(wquad, 0, 2, " Quadrant %d - %d ", quadx, quady);
		/* srscan() heads the grid with SHORT-RANGE SENSORS DAMAGED;
		   the panel has no line to spare for that, so it says the
		   same thing on the bottom border. Without it the dashes
		   are a field of nothing with no reason given. Seventeen
		   columns inside a twenty-nine-column box, so it fits at
		   the 72x24 minimum as well as anywhere else -- but not
		   below 20, where the window itself is narrower than that
		   and the caption ate the bottom-right corner.

		   The row is asked of the window rather than named: it is
		   the bottom border wherever that has ended up, which on a
		   terminal too short for the panel is not PANELH-1. Naming
		   it meant the dashes lost their explanation from twelve
		   rows down -- the field of nothing this exists to prevent,
		   at the sizes where a player is least sure what they are
		   looking at. */
		if (sensors_masked() && getmaxy(wquad) >= 2 &&
		    panel_room(wquad) >= (int)strlen(" SENSORS DAMAGED "))
			mvwaddstr(wquad, getmaxy(wquad)-1, 2,
				  " SENSORS DAMAGED ");
		for (i = 0; i <= 10; i++) {
			fmt_quad_line(i, buf);
			draw_quad_line(i+1, buf);
		}
		for (i = 1; i <= 10; i++) {
			fmt_status_line(i, buf);
			draw_status_line(i+1, i, buf);
		}
	}
	wnoutrefresh(wquad);
	wnoutrefresh(wstat);
	wnoutrefresh(wmsg);
	doupdate();
}

void tui_puts(char *s) {
	note_out(s);
	waddstr(wmsg, s);
	wrefresh(wmsg);
}

void tui_puts_slow(char *s) {
	note_out(s);
	while (*s) {
		waddch(wmsg, *s++);
		wrefresh(wmsg);
		napms(30);
	}
}

/* Rub out the character before the cursor, on screen.
 *
 * An answer long enough to wrap puts the cursor at column 0 with the
 * character to delete at the end of the row above, so that case steps
 * back a row rather than giving up -- giving up is what let the buffer
 * and the screen disagree, the player rubbing out characters that
 * stayed visible while the game forgot them.
 *
 * Only ever called with something still to rub out, which is what keeps
 * it off the prompt: it deletes no more characters than were typed. */
static void erase_one(void) {
	int y, x;

	getyx(wmsg, y, x);
	if (x > 0) {
		mvwaddch(wmsg, y, x-1, ' ');
		wmove(wmsg, y, x-1);
	} else if (y > 0) {
		x = getmaxx(wmsg) - 1;
		mvwaddch(wmsg, y-1, x, ' ');
		wmove(wmsg, y-1, x);
	}
}

/* Read a line, a keystroke at a time, rather than with wgetnstr.
 *
 * Two things need that. A resize has to be able to rebuild the windows
 * and carry on waiting, which cannot be done underneath wgetnstr -- it
 * is still reading the screen curses would be repainting, which is what
 * made every attempt at it blank the display. Here the resize arrives
 * between reads, where a repaint is safe, exactly as in tui_getch().
 *
 * And cbreak() turns off the tty's own end-of-file handling, so Ctrl-D
 * arrives as a character. wgetnstr returns only on Enter, so it used to
 * take a Ctrl-D *and* an Enter to end a session. On its own keystroke
 * now, as everywhere else. */
int tui_readline(char *buf, int buflen) {
	int len = 0, room = buflen-2, c;

	if (room < 0) room = 0;	/* keep room for the "\n" and the NUL */
	reader_waiting = TRUE;
	tui_refresh_panels();
	for (;;) {
		c = wgetch(wmsg);
		if (c == KEY_RESIZE) {
			/* Written whenever the repaint moved the cursor,
			   rather than only when it decided the line had
			   been lost. It leaves the cursor at the end of the
			   prompt, which is where an echo belongs: where the
			   answer survived on screen this writes the same
			   characters over themselves, and where it did not
			   this puts them there. Either way the screen ends
			   up saying exactly what buf holds. The repaint runs
			   whichever way the terminal moved, so the guard is
			   not about the direction: it is for a line the
			   repaint could not place at all, where the cursor
			   is wherever the game left it and an echo would
			   land in the wrong column.
			   Doing it only when the line had been reprinted
			   left the cursor in front of an answer that was
			   already drawn, so the next keystroke typed over
			   it -- srsc then an came out "ansc" and ran
			   srscan, a command the screen never showed. */
			buf[len] = '\0';
			cursor_at_prompt = FALSE;
			pending_answer = buf;
			tui_refresh_panels();
			pending_answer = NULL;
			if (cursor_at_prompt && len > 0) {
				waddstr(wmsg, buf);
				wrefresh(wmsg);
			}
			continue;
		}
		if (c == ERR)
			break;			/* input has ended */
		if (c == '\n' || c == '\r' || c == KEY_ENTER) {
			waddch(wmsg, '\n');
			/* Ends the line for note_out as well, which only
			   ever sees what the game writes. Without this the
			   answer's own newline goes unnoticed and the line
			   being remembered grows across prompts, so the
			   restore looks for a string spanning two of them,
			   never finds it, and writes a second copy. */
			note_out("\n");
			wrefresh(wmsg);
			buf[len] = 0;
			strcat(buf, "\n");	/* the fgets-like contract;
						   readinput() strips it */
			reader_waiting = FALSE;
			return TRUE;
		}
		if (c == '\004') {		/* Ctrl-D */
			if (len == 0) break;	/* nothing typed: end of input */
			continue;		/* mid-line a tty ignores it */
		}
		if (c == KEY_BACKSPACE || c == '\b' || c == 127) {
			if (len > 0) {
				len--;
				erase_one();
				wrefresh(wmsg);
			}
			continue;
		}
		if (c == '\025') {		/* Ctrl-U */
			/* The tty's own kill character, which wgetnstr
			   used to honour. It is the reflex for "scrap that
			   and start again" at any prompt, and the one piece
			   of line editing the old reader had that this one
			   would otherwise have dropped. */
			while (len > 0) {
				len--;
				erase_one();
			}
			wrefresh(wmsg);
			continue;
		}
		/* Printable ASCII only. The game asks for words and numbers,
		   and a stray control code or an arrow key answering a
		   question is worse than one the player has to type again. */
		if (c >= ' ' && c < 127 && len < room) {
			buf[len++] = (char)c;
			waddch(wmsg, c);
			wrefresh(wmsg);
		}
	}
	buf[0] = 0;
	reader_waiting = FALSE;
	return FALSE;			/* no more input is coming */
}

int tui_getch(void) {
	int c;

	/* Same as before a typed answer: a paging prompt is a moment the
	   player is looking at the screen, so the panels beside the text
	   should not be older than it. */
	reader_waiting = TRUE;
	tui_refresh_panels();
	for (;;) {
		c = wgetch(wmsg);
		/* Not a keystroke, whatever curses calls it. Handing it back
		   would let a window drag answer "hit space bar to continue"
		   and page away text the player never read. */
		if (c != KEY_RESIZE) {
			reader_waiting = FALSE;
			return c;
		}
		/* Redraw for the new size before waiting again. Safe
		   because curses reports the resize by returning from
		   wgetch, so nothing is reading the screen at this moment
		   -- the same reason it is safe in tui_readline above, now
		   that it reads a keystroke at a time. Leaving it until the
		   next prompt is what made a drag mid-pause look like a
		   game that had stopped: the panels stayed the old width
		   with the new columns empty beside them, for as long as
		   the player took to press a key. */
		tui_refresh_panels();
	}
}

void tui_clearmsg(void) {
	werase(wmsg);
	curlen = 0;		/* nothing is part written any more */
	curline[0] = '\0';
	wrefresh(wmsg);
	lastline[0] = '\0';	/* erased on purpose; do not resurrect it */
	lastlen = 0;
}

int tui_pageheight(void) {
	return LINES-PANELH-1;
}
