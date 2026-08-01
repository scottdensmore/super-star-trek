/* Full-screen curses interface (sst -t).
 *
 * The pure formatting functions live in tuifmt.c so they can be unit
 * tested without curses; the curses backend itself lives in tui.c.
 */

#define FMTBUFLEN (48)	/* enough for any panel line */
#define STATLABEL (14)	/* width fmt_status_line pads its labels to;
			   where tui.c starts colouring the value */

/* What a grid cell and a status line are, for colouring.
 *
 * A cell is classified from the character the panel is about to show,
 * never from the game state behind it. When the sensors are damaged
 * every hidden cell reads as '-', so they all classify alike and
 * colour cannot become a way to see what a scan is hiding. */
enum cellclass {
	CELL_PLAIN,	/* empty space, grid labels */
	CELL_HOSTILE,	/* Klingons, Commanders, Romulans, Tholians */
	CELL_SHIP,	/* ours */
	CELL_BASE,
	CELL_STAR,
	CELL_PLANET,
	CELL_WEB,
	CELL_THING,	/* the unknown object Spock tells you to look at */
	CELL_HIDDEN	/* masked by damaged sensors */
};

enum statusclass {
	STAT_PLAIN,
	STAT_GOOD,
	STAT_WARN,
	STAT_BAD,
	STAT_DOCKED
};

/* Panel formatters (tuifmt.c). Both read the global game state. */
void fmt_quad_line(int i, char *buf);	/* i=0 header, 1..10 grid rows */
void fmt_status_line(int i, char *buf);	/* i=1..10, srscan status fields */
int cell_class(char c);			/* colour class of a shown cell */
int status_class(int i);		/* colour class of a status line */

/* Curses backend (tui.c) */
EXTERN int tui_active;	/* TUI mode is on and curses is running */
EXTERN int tui_ingame;	/* a game is set up, so the panels have something
			   to show; false through the setup questions,
			   including a second game's */

int stdio_is_terminal(void);	/* osx.c; stdin and stdout are both ttys */

int tui_init(void);		/* returns TRUE if the TUI started */
int tui_terminal_capable(void);	/* TERM is known and can address the cursor */
void tui_shutdown(void);
void tui_puts(char *s);
void tui_puts_slow(char *s);
int tui_readline(char *buf, int buflen);	/* fgets-like: keeps '\n';
						   FALSE when input has ended */
int tui_getch(void);
void tui_clearmsg(void);
int tui_pageheight(void);	/* usable message-window lines for paging */
void tui_refresh_panels(void);
