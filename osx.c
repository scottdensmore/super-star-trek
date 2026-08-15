/* osx.h and tui.h, not sst.h: the first declares what this file
   provides, the second what getch() calls, and neither drags in
   sst.h's pause(int), which collides with the POSIX pause(void) that
   <unistd.h> below brings in. osx.h goes first so the build would
   notice if it ever stopped standing on its own. */
#include "osx.h"
#include "tui.h"
#include <stdio.h>
#ifndef WINDOWS
#include <errno.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>
#endif

/* randomize() used to live here, next to srand(). The game has its own
   generator now (see Rand() in sst.c), and seeding it is not a
   platform matter, so it moved there. */

#ifndef WINDOWS
int max(int a, int b) {
	if (a > b) return a;
	return b;
}

int min(int a, int b) {
	if (a < b) return a;
	return b;
}

/* Whether the game is talking to a real terminal. This lives here
   rather than in tui.c because it needs <unistd.h>, and sst.h -- which
   tui.c includes -- declares void pause(int), which collides with the
   POSIX pause(void) that header brings in. Declared in osx.h with the
   rest of what this file provides. */
int stdio_is_terminal(void) {
	return isatty(STDIN_FILENO) && isatty(STDOUT_FILENO);
}

int getch(void) {
    char chbuf[1];
    ssize_t n;
    struct termios oldstate, newstate;
    if (tui_active)
        return tui_getch();
    fflush(stdout);
	tcgetattr(0, &oldstate);
	newstate = oldstate;
	newstate.c_lflag &= ~ICANON;
	newstate.c_lflag &= ~ECHO;
	tcsetattr(0, TCSANOW,  &newstate);
	/* A read the terminal changing shape interrupted is not a
	   keypress, and reporting one lets a resize answer the pause: the
	   page the player had not finished goes by, and at pause(1) the
	   clearscreen() after it takes the screen with it. Same cause as
	   readinput()'s, and sst.doc promises against both in one
	   sentence -- resizing is safe "while the game waits for an
	   answer or for a keystroke at a pause". Issue #150.
	   At end of input there is no keypress to report; say so rather
	   than handing back whatever was on the stack. Play continues
	   here, unlike readinput(), which ends the session: this is the
	   paging prompt, and quitting from it would cut off end-of-game
	   output mid-score. The session ends at the next real prompt. */
	do {
		n = read(0, &chbuf, 1);
	} while (n < 0 && errno == EINTR);
	if (n != 1)
		chbuf[0] = '\0';
	tcsetattr(0, TCSANOW, &oldstate);
        return chbuf[0];
}
#endif