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

/* Draw a grid line a cell at a time so each symbol carries its own
 * colour. Row labels and spacing classify as plain and stay uncoloured. */
static void draw_quad_line(int row, const char *s) {
	int i;

	wmove(wquad, row, 2);
	for (i = 0; s[i] != '\0'; i++) {
		chtype a = attr_for_cell(s[i]);
		if (a != A_NORMAL) wattron(wquad, a);
		waddch(wquad, (unsigned char)s[i]);
		if (a != A_NORMAL) wattroff(wquad, a);
	}
}

/* The label stays plain and the value takes the colour, so a glance
 * down the panel picks out the fields that are shouting. */
static void draw_status_line(int row, int line, const char *s) {
	chtype a = attr_for_status(line);
	int i;

	wmove(wstat, row, 2);
	for (i = 0; s[i] != '\0'; i++) {
		if (i == STATLABEL && a != A_NORMAL) wattron(wstat, a);
		waddch(wstat, (unsigned char)s[i]);
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
	int msgh, statw, msgw;

	builtlines = LINES;
	builtcols = COLS;
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
		wresize(wstat, PANELH, statw);
		wresize(wmsg, msgh, msgw);
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

static void note_out(const char *s) {
	for (; *s != '\0'; s++) {
		/* A carriage return ends it as surely as a newline does:
		   pause() wipes its own prompt with "\r ... \r", and
		   keeping those bytes left curline holding 56 characters
		   that render as an empty row, which then matched nothing
		   on screen. */
		if (*s == '\n' || *s == '\r')
			curlen = 0;
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
 * with the answer to be written after it. False when the restore was
 * not needed at all -- a terminal that only grew -- where the cursor is
 * still sitting after the answer and writing it again would double it. */
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
	char *found;
	int r, last, maxy, width, i, tail_blank, wantlen;

	if (curlen == 0) return;
	/* What the bottom of the conversation should read: the line the
	   game is part way through, and after it whatever the reader has
	   echoed of the answer. Comparing against the prompt alone made
	   every pending answer look like something drawn over the line,
	   so an ordinary drag reprinted the prompt at each step and stood
	   eight copies of it up the window. */
	snprintf(want, sizeof want, "%s%s", curline,
		 pending_answer != NULL ? pending_answer : "");
	wantlen = (int)strlen(want);
	maxy = getmaxy(wmsg);
	width = getmaxx(wmsg);
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
				i = (int)(found - joined) + curlen;
				wmove(wmsg, start + i / width, i % width);
				cursor_at_prompt = TRUE;
				return;
			}
		}
	}
	/* Where to put it. A terminal that lost columns left the stump on
	   screen, taking as many rows as the pair filled at the old
	   width -- which is arithmetic, not something to go looking for.
	   Rub those rows out and write the line in their place; scrolling
	   instead is what left one stump per step of a narrowing drag,
	   six of them up the window with the conversation pushed off the
	   top.
	   A terminal that only lost rows is the other case: the line is
	   gone altogether and the bottom rows hold older conversation
	   worth keeping, so that scrolls by one and writes below it. */
	if (oldmsgw > 0) {
		int total = curlen;
		int rows, rr;

		if (pending_answer != NULL)
			total += (int)strlen(pending_answer);
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
		wscrl(wmsg, 1);
		wmove(wmsg, maxy - 1, 0);
	}
	waddstr(wmsg, curline);
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
	int oldlines = builtlines, oldcols = builtcols;

	if (!resized()) return;
	cursor_at_prompt = FALSE;
	resize_term(0, 0);	/* adopt whatever the terminal is now */
	make_windows();		/* which is what moves builtlines/builtcols on */
	touchwin(wmsg);		/* the panels redraw themselves; this does not */
	/* A terminal that only grew clipped nothing, so there is nothing
	   to put back and no need to go looking: asking anyway is what
	   left a second copy of a question whenever the answer being
	   typed was long enough to wrap, since a wrapped pair is not the
	   contiguous string the search hunts for. */
	if (LINES < oldlines || COLS < oldcols)
		restore_curline(COLS < oldcols ? oldcols - 2 : 0);
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
	mvwaddstr(wquad, 0, 2, " Quadrant ");
	werase(wstat);
	box(wstat, 0, 0);
	mvwaddstr(wstat, 0, 2, " Status ");
	/* Nothing to show before a game is set up or after one ends; the
	   formatters work the condition out for themselves, so nothing
	   here writes to the game state. */
	if (tui_ingame) {
		mvwprintw(wquad, 0, 2, " Quadrant %d - %d ", quadx, quady);
		/* srscan() heads the grid with SHORT-RANGE SENSORS DAMAGED;
		   the panel has no line to spare for that, so it says the
		   same thing on the bottom border. Without it the dashes
		   are a field of nothing with no reason given. Seventeen
		   columns inside a twenty-nine-column box, so it fits at
		   the 72x24 minimum as well as anywhere else. */
		if (sensors_masked())
			mvwaddstr(wquad, PANELH-1, 2, " SENSORS DAMAGED ");
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
			   up saying exactly what buf holds -- and where the
			   repaint did nothing at all, because the terminal
			   only grew, the cursor is still after the answer
			   and writing it again would double it.
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
	return FALSE;			/* no more input is coming */
}

int tui_getch(void) {
	int c;

	/* Same as before a typed answer: a paging prompt is a moment the
	   player is looking at the screen, so the panels beside the text
	   should not be older than it. */
	tui_refresh_panels();
	for (;;) {
		c = wgetch(wmsg);
		/* Not a keystroke, whatever curses calls it. Handing it back
		   would let a window drag answer "hit space bar to continue"
		   and page away text the player never read. */
		if (c != KEY_RESIZE) return c;
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
}

int tui_pageheight(void) {
	return LINES-PANELH-1;
}
