/* Full-screen curses interface (sst -t).
 *
 * The pure formatting functions live in tuifmt.c so they can be unit
 * tested without curses; the curses backend itself lives in tui.c.
 */

#define FMTBUFLEN (48)	/* enough for any panel line */

/* Panel formatters (tuifmt.c). Both read the global game state. */
void fmt_quad_line(int i, char *buf);	/* i=0 header, 1..10 grid rows */
void fmt_status_line(int i, char *buf);	/* i=1..10, srscan status fields */

/* Curses backend (tui.c) */
EXTERN int tui_active;	/* TUI mode is on and curses is running */

int tui_init(void);		/* returns TRUE if the TUI started */
void tui_shutdown(void);
void tui_puts(char *s);
void tui_puts_slow(char *s);
void tui_readline(char *buf, int buflen);	/* fgets-like: keeps '\n' */
int tui_getch(void);
void tui_clearmsg(void);
int tui_pageheight(void);	/* usable message-window lines for paging */
void tui_refresh_panels(void);
