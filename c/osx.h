/* What osx.c provides: the few things the game needs from the
 * platform rather than from itself.
 *
 * This header exists because osx.c cannot include sst.h -- sst.h
 * declares a pause(int) that collides with the POSIX pause(void) that
 * <unistd.h> brings in, and osx.c needs the POSIX one. Without a
 * header both sides can see, these definitions were checked against
 * nothing: a signature could drift from the one callers read and
 * nothing -- not the compiler, not the linker, which does not carry
 * signatures at all -- would say so. sst.h includes this file, so
 * callers keep reaching them the way they always have.
 *
 * Self-contained on purpose, like tui.h: including this must not
 * oblige a file to include sst.h as well.
 */
#ifndef SST_OSX_H
#define SST_OSX_H

/* Windows has its own min and max -- MSVC's <stdlib.h> makes them
   macros -- so osx.c defines none there. The other two stay declared
   unconditionally, as they were before this file existed (getch() in
   sst.h, stdio_is_terminal() in tui.h): sst.c calls getch() without
   including <conio.h>, and taking the declaration away would leave it
   with an implicit one. On WINDOWS osx.c defines neither, so both
   declarations stand without a definition behind them -- that build
   has never been wired into CMake, so nothing checks either way. */
#ifndef WINDOWS
int min(int, int);
int max(int, int);
#endif
int getch(void);
int stdio_is_terminal(void);	/* stdin and stdout are both ttys */

#endif	/* SST_OSX_H */
