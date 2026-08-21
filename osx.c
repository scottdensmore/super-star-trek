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
#include <fcntl.h>
#include <signal.h>
#include <sys/ioctl.h>
#include <sys/select.h>
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

/* Wake pselect() through a nonblocking pipe after job control resumes the
   game, so getch() can restore the mode the shell replaced while stopped. */
static volatile sig_atomic_t getch_wake_fd = -1;
static void interrupt_getch(int signum) {
	int saved_errno = errno;
	unsigned char wake = 0;

	(void)signum;
	if (getch_wake_fd >= 0) {
		ssize_t written = write((int)getch_wake_fd, &wake, sizeof(wake));
		(void)written;
	}
	errno = saved_errno;
}

int getch(void) {
	char chbuf[1];
	ssize_t n = -1;
	int havecont = 0, havemask = 0, havepipe = 0, haveterm, pipeflags;
	int wakepipe[2] = {-1, -1};
	fd_set readfds;
	sigset_t blockjob, oldmask, waitmask;
	struct sigaction oldcont, wakecont = {0};
	struct termios oldstate, newstate;
	if (tui_active)
		return tui_getch();
	fflush(stdout);
	haveterm = tcgetattr(STDIN_FILENO, &oldstate) == 0;
	if (!haveterm) {
		do {
			n = read(STDIN_FILENO, &chbuf, 1);
		} while (n < 0 && errno == EINTR);
		if (n != 1)
			chbuf[0] = '\0';
		return chbuf[0];
	}
	newstate = oldstate;
	newstate.c_lflag &= ~ICANON;
	newstate.c_lflag &= ~ECHO;
	sigemptyset(&blockjob);
	sigaddset(&blockjob, SIGCONT);
	sigaddset(&blockjob, SIGTSTP);
	wakecont.sa_handler = interrupt_getch;
	sigemptyset(&wakecont.sa_mask);
	if (sigprocmask(SIG_BLOCK, &blockjob, &oldmask) == 0) {
		havemask = 1;
		waitmask = oldmask;
		sigdelset(&waitmask, SIGCONT);
		sigdelset(&waitmask, SIGTSTP);
		if (pipe(wakepipe) == 0) {
			havepipe = 1;
			if (wakepipe[0] >= FD_SETSIZE) {
				close(wakepipe[0]);
				close(wakepipe[1]);
				wakepipe[0] = wakepipe[1] = -1;
				havepipe = 0;
			} else {
				pipeflags = fcntl(wakepipe[1], F_GETFL);
				if (pipeflags >= 0 &&
				    fcntl(wakepipe[1], F_SETFL,
				          pipeflags | O_NONBLOCK) == 0) {
					getch_wake_fd = wakepipe[1];
					havecont = sigaction(SIGCONT, &wakecont,
					                     &oldcont) == 0;
				}
			}
		}
	}
	/* A read the terminal changing shape interrupted is not a
	   keypress, and reporting one lets a resize answer the pause: the
	   page the player had not finished goes by, and at pause(1) the
	   clearscreen() after it takes the screen with it. Same cause as
	   readinput()'s, and sst.doc promises against both in one
	   sentence -- resizing is safe "while the game waits for an
	   answer or for a keystroke at a pause". Issue #150.
	   No resize reaches here since #152 put the SIGWINCH disposition
	   back. #158 did the same for SIGTSTP, so this code -- which runs
	   whenever the panels are not up, in a plain game and in one that
	   fell back alike -- meets the stop disposition the game had before
	   curses, not curses' handler, on the path where curses ran at all.
	   The reason given here until #158 was that curses' SIGTSTP
	   handler carried SA_RESTART; there is no curses handler on this
	   path any more. Without the scoped handler below, a default stop
	   and continue restarts the read in place. Measured on the #158
	   branch: a pause survives a suspend and is still waiting on resume,
	   but the shell has restored
	   canonical mode while the read is being restarted in the kernel.
	   Space then sits in the line discipline until Enter releases it.
	   SIGTSTP and SIGCONT are blocked from before the mode change until
	   pselect() atomically replaces the mask while it waits. A stop and
	   resume before or during that wait either interrupts pselect() or
	   makes its wake pipe readable, and comes back through tcsetattr().
	   The pipe is the macOS half: both PR #200 macOS configurations left
	   the handler-only pselect() waiting after resume, while the same code
	   passed on Linux. After input is ready, both
	   signals are blocked again until read() takes that byte. This closes
	   both gaps around the wait. The old disposition and mask are
	   restored below, so only a classic getch() has this behavior. #190.
	   Plain mode needs the same fix as fallback.
	   The retry stays anyway: reading an interrupted read as a
	   keypress is wrong whatever the signals happen to be.
	   At end of input there is no keypress to report; say so rather
	   than handing back whatever was on the stack. Play continues
	   here, unlike readinput(), which ends the session: this is the
	   paging prompt, and quitting from it would cut off end-of-game
	   output mid-score. The session ends at the next real prompt. */
	while (havecont) {
		if (tcsetattr(STDIN_FILENO, TCSANOW, &newstate) != 0) {
			if (errno == EINTR)
				continue;
			break;
		}
		FD_ZERO(&readfds);
		FD_SET(STDIN_FILENO, &readfds);
		FD_SET(wakepipe[0], &readfds);
		n = pselect(wakepipe[0] + 1, &readfds, NULL, NULL, NULL,
		            &waitmask);
		if (n <= 0) {
			if (n < 0 && errno == EINTR)
				continue;
			break;
		}
		if (FD_ISSET(wakepipe[0], &readfds)) {
			unsigned char wake;
			do {
				n = read(wakepipe[0], &wake, sizeof(wake));
			} while (n < 0 && errno == EINTR);
			continue;
		}
		n = read(STDIN_FILENO, &chbuf, 1);
		if (n < 0 && errno == EINTR)
			continue;
		break;
	}
	if (n != 1)
		chbuf[0] = '\0';
	tcsetattr(STDIN_FILENO, TCSANOW, &oldstate);
	if (havecont)
		sigaction(SIGCONT, &oldcont, NULL);
	getch_wake_fd = -1;
	if (havepipe) {
		close(wakepipe[0]);
		close(wakepipe[1]);
	}
	if (havemask)
		sigprocmask(SIG_SETMASK, &oldmask, NULL);
	return chbuf[0];
}
#endif
