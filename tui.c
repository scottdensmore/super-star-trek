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
	wquad = newwin(PANELH, QUADW, 0, 0);
	wstat = newwin(PANELH, COLS-QUADW, 0, QUADW);
	wmsg = newwin(LINES-PANELH, COLS-2, PANELH, 1);
	scrollok(wmsg, TRUE);
	keypad(wmsg, TRUE);
	tui_active = TRUE;
	atexit(tui_shutdown);
	tui_refresh_panels();
	return TRUE;
}

void tui_shutdown(void) {
	if (!tui_active) return;
	tui_active = FALSE;
	endwin();
}

void tui_refresh_panels(void) {
	char buf[FMTBUFLEN];
	int i;

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
	waddstr(wmsg, s);
	wrefresh(wmsg);
}

void tui_puts_slow(char *s) {
	while (*s) {
		waddch(wmsg, *s++);
		wrefresh(wmsg);
		napms(30);
	}
}

int tui_readline(char *buf, int buflen) {
	int got;

	tui_refresh_panels();
	echo();
	got = wgetnstr(wmsg, buf, buflen-2) != ERR;
	noecho();
	/* cbreak() turns off canonical mode, and with it the tty's own
	   end-of-file handling, so Ctrl-D arrives as a plain character
	   rather than as ERR. A line holding nothing else still means end
	   of input. wgetnstr only returns on Enter, so unlike a normal
	   terminal the Ctrl-D has to be followed by one; ending the
	   session properly on the keystroke alone would mean replacing
	   wgetnstr with our own line editor. */
	if (got && buf[0] == '\004' && buf[1] == 0)
		got = FALSE;
	if (!got) {
		buf[0] = 0;
		return FALSE;	/* no more input is coming */
	}
	strcat(buf, "\n");	/* scan() expects fgets-style input */
	return TRUE;
}

int tui_getch(void) {
	/* Same as before a typed answer: a paging prompt is a moment the
	   player is looking at the screen, so the panels beside the text
	   should not be older than it. */
	tui_refresh_panels();
	return wgetch(wmsg);
}

void tui_clearmsg(void) {
	werase(wmsg);
	wrefresh(wmsg);
}

int tui_pageheight(void) {
	return LINES-PANELH-1;
}
