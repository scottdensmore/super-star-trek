#include <stdlib.h>
#include <stdio.h>
#include <time.h>
#ifndef WINDOWS
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>
#endif

void randomize(void) {
	srand((int)time(NULL));
}

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
   POSIX pause(void) that header brings in. */
int stdio_is_terminal(void) {
	return isatty(STDIN_FILENO) && isatty(STDOUT_FILENO);
}

int getch(void) {
    char chbuf[1];
    struct termios oldstate, newstate;
    extern int tui_active;
    int tui_getch(void);
    if (tui_active)
        return tui_getch();
    fflush(stdout);
	tcgetattr(0, &oldstate);
	newstate = oldstate;
	newstate.c_lflag &= ~ICANON;
	newstate.c_lflag &= ~ECHO;
	tcsetattr(0, TCSANOW,  &newstate);
	/* At end of input there is no keypress to report; say so rather
	   than handing back whatever was on the stack. Play continues
	   here, unlike readinput(), which ends the session: this is the
	   paging prompt, and quitting from it would cut off end-of-game
	   output mid-score. The session ends at the next real prompt. */
	if (read(0, &chbuf, 1) != 1)
		chbuf[0] = '\0';
	tcsetattr(0, TCSANOW, &oldstate);
        return chbuf[0];
}
#endif