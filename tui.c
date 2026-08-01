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
	if (d.date > 0.0) {	/* nothing to show until the game is set up */
		if (condit != IHDOCKED)
			newcnd();	/* srscan does the same before showing Condition */
		mvwprintw(wquad, 0, 2, " Quadrant %d - %d ", quadx, quady);
		for (i = 0; i <= 10; i++) {
			fmt_quad_line(i, buf);
			mvwaddstr(wquad, i+1, 2, buf);
		}
		for (i = 1; i <= 10; i++) {
			fmt_status_line(i, buf);
			mvwaddstr(wstat, i+1, 2, buf);
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
	wrefresh(wmsg);
	return wgetch(wmsg);
}

void tui_clearmsg(void) {
	werase(wmsg);
	wrefresh(wmsg);
}

int tui_pageheight(void) {
	return LINES-PANELH-1;
}
