#include "sst.h"
#include "tui.h"
#include <curses.h>

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

int tui_init(void) {
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

void tui_readline(char *buf, int buflen) {
	tui_refresh_panels();
	echo();
	if (wgetnstr(wmsg, buf, buflen-2) == ERR)
		buf[0] = 0;
	noecho();
	strcat(buf, "\n");	/* scan() expects fgets-style input */
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
