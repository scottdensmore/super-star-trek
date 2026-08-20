#define INCLUDED	// Define externs here
#include "sst.h"
#include <errno.h>
#include <stdint.h>
#include "tui.h"
#include <ctype.h>
#include <stdarg.h>
#ifdef MSDOS
#include <dos.h>
#endif
#include <time.h>

static char line[128], *linep = line;
static int linecount;	/* for paging */
/* The game has asked the player for a line at least once. Until then
   it is still introducing itself -- and with the setup answers given
   on the command line it reaches the briefing without the player
   having touched the keyboard at all. See skip(). */
static int prompted;

static void clearscreen(void);

/* Compared to original version, I've changed the "help" command to
   "call" and the "terminate" command to "quit" to better match
   user expectations. The DECUS version apparently made those changes
   as well as changing "freeze" to "save". However I like "freeze".

   When I got a later version of Super Star Trek that I was converting
   from, I added the emexit command.

   That later version also mentions srscan and lrscan working when
   docked (using the starbase's scanners), so I made some changes here
   to do this (and indicating that fact to the player), and then realized
   the base would have a subspace radio as well -- doing a Chart when docked
   updates the star chart, and all radio reports will be heard. The Dock
   command will also give a report if a base is under attack.

   Movecom no longer reports movement if sensors are damaged so you wouldn't
   otherwise know it.

   Also added:

   1. Better base positioning at startup

   2. deathray improvement (but keeping original failure alternatives)

   3. Tholian Web

   4. Enemies can ram the Enterprise. Regular Klingons and Romulans can
      move in Expert and Emeritus games. This code could use improvement.

   5. The deep space probe looks interesting! DECUS version

   6. Cloaking (with contributions from Erik Olofsen) and Capturing (BSD version).

   */

// I don't like the way this is done, relying on an index. But I don't
// want to invest the time to make this nice and table driven.

static char *commands[] = {
	"srscan",
	"lrscan",
	"phasers",
	"photons",
	"move",
	"shields",
	"dock",
	"damages",
	"chart",
	"impulse",
	"rest",
	"warp",
	"status",
	"sensors",
	"orbit",
	"transport",
	"mine",
	"crystals",
	"shuttle",
	"planets",
	"request",
	"report",
	"computer",
	"commands",
    "emexit",
    "probe",
    "cloak",
    "capture",
    "score",
	"abandon",
	"destruct",
	"freeze",
	"deathray",
	"debug",
	"call",
	"quit",
    "help"
 
};

/* Signed, because every loop below counts through it with an int. */
#define NUMCOMMANDS ((int)(sizeof(commands)/sizeof(char *)))

static void listCommands(int x) {
	prout("   SRSCAN    MOVE      PHASERS   CALL\n"
		  "   STATUS    IMPULSE   PHOTONS   ABANDON\n"
		  "   LRSCAN    WARP      SHIELDS   DESTRUCT\n"
		  "   CHART     REST      DOCK      QUIT\n"
		  "   DAMAGES   REPORT    SENSORS   ORBIT\n"
		  "   TRANSPORT MINE      CRYSTALS  SHUTTLE\n"
		  "   PLANETS   REQUEST   DEATHRAY  FREEZE\n"
          "   COMPUTER  EMEXIT    PROBE     COMMANDS");
    proutn("   ");
#ifdef SCORE
    proutn("SCORE     ");
#endif
#ifdef CLOAKING
    proutn("CLOAK     ");
#endif
#ifdef CAPTURE
    proutn("CAPTURE   ");
#endif
    if (x) proutn("HELP     ");
    prout("");
}

static void helpme(void) {
	int i, j;
	char cmdbuf[32];
	char linebuf[132];
	FILE *fp;
	/* Give help on commands */
	int key;
	key = scan();
	while (TRUE) {
		if (key == IHEOL) {
			proutn("Help on what command?");
			key = scan();
		}
		if (key == IHEOL) return;
		for (i = 0; i < NUMCOMMANDS; i++) {
			if (strcmp(commands[i], citem)==0) break;
		}
		if (i != NUMCOMMANDS) break;
		skip(1);
		prout("Valid commands:");
		listCommands(FALSE);
		key = IHEOL;
		chew();
		skip(1);
	}
	if (i == 23) {
		strcpy(cmdbuf, " ABBREV");
	}
	else {
		strcpy(cmdbuf, "  Mnemonic:  ");
		j = 0;
		while ((cmdbuf[j+13] = toupper(commands[i][j])) != 0) j++;
	}
	fp = fopen("sst.doc", "r");
	if (fp == NULL) {
		prout("Spock-  \"Captain, that information is missing from the");
		prout("   computer. You need to find SST.DOC and put it in the");
		prout("   current directory.\"");
		return;
	}
	i = strlen(cmdbuf);
	do {
		if (fgets(linebuf, 132, fp) == NULL) {
			prout("Spock- \"Captain, there is no information on that command.\"");
			fclose(fp);
			return;
		}
	} while (strncmp(linebuf, cmdbuf, i) != 0);

	skip(1);
	prout("Spock- \"Captain, I've found the following information:\"");
	skip(1);

	do {
		if (linebuf[0]!=12) { // ignore page break lines 
			/* sst.doc has CRLF endings. Dropping only the \n
			   leaves a \r, and in the full-screen display that
			   returns the cursor to column 0 so the following
			   newline wipes the line that was just drawn --
			   which is why help used to come out blank there. */
			char *end = linebuf + strlen(linebuf);
			while (end > linebuf && (end[-1] == '\n' || end[-1] == '\r'))
				*--end = '\0';
			prout(linebuf);
		}
		/* A topic with no ****** terminator would otherwise
		   reprint its last line for ever. Say so, the way the
		   other two ways this can fail already do. */
		if (fgets(linebuf,132,fp) == NULL) {
			prout("Spock- \"Captain, the rest of that entry is missing from the computer.\"");
			break;
		}
	} while (strstr(linebuf, "******")==NULL);
	fclose(fp);
}

static void makemoves(void) {
    int i, hitme;
    while (TRUE) { /* command loop */
        hitme = FALSE;
        justin = 0;
        Time = 0.0;
        i = -1;
        while (TRUE) { /* get a command */
            int matched = 0;
            chew();
            skip(1);
            proutn("COMMAND> ");
            if (scan() == IHEOL) continue;
            for (i = 0; i < 29; i++) // Abbreviations allowed for the first 29 commands, only.
                if (isit(commands[i]))
                    break;
            if (i < 29) {
                matched = 1;
            } else {
                for (; i < NUMCOMMANDS; i++)
                    if (strcmp(commands[i], citem) == 0) {
                        matched = 1;
                        break;
                    }
            }
            if (matched
#ifndef CLOAKING
                    && i != 26 // ignore the CLOAK command
#endif
#ifndef CAPTURE
                    && i != 27 // ignore the CAPTURE command
#endif
#ifndef SCORE
                    && i != 28 // ignore the SCORE command
#endif
#ifndef DEBUG
                    && i != 33 // ignore the DEBUG command
#endif
                    ) break;

            if (skill <= SFAIR) {
                prout("UNRECOGNIZED COMMAND. LEGAL COMMANDS ARE:");
                listCommands(TRUE);
            } else prout("UNRECOGNIZED COMMAND.");
        }
        switch (i) { /* command switch */
            case 0: // srscan
                srscan(1);
                break;
            case 1: // lrscan
                lrscan();
                break;
            case 2: // phasers
                phasers();
                if (ididit) {
#ifdef CLOAKING
                    if (irhere && d.date >= ALGERON && !isviolreported && iscloaked) {
                        prout("The Romulan ship discovers you are breaking the Treaty of Algeron!");
                        ncviol++;
                        isviolreported = TRUE;
                    }
#endif
                    hitme = TRUE;
                }
                break;
            case 3: // photons
                photon();
                if (ididit) {
#ifdef CLOAKING
                    if (irhere && d.date >= ALGERON && !isviolreported && iscloaked) {
                        prout("The Romulan ship discovers you are breaking the Treaty of Algeron!");
                        ncviol++;
                        isviolreported = TRUE;
                    }
#endif
                    hitme = TRUE;
                }
                break;
            case 4: // move
                warp(1);
                break;
            case 5: // shields
                sheild(1);
                if (ididit) {
                    attack(2);
                    shldchg = 0;
                }
                break;
            case 6: // dock
                dock();
                break;
            case 7: // damages
                dreprt();
                break;
            case 8: // chart
                chart(0);
                break;
            case 9: // impulse
                impuls();
                break;
            case 10: // rest
                waiting();
                if (ididit) hitme = TRUE;
                break;
            case 11: // warp
                setwrp();
                break;
            case 12: // status
                srscan(3);
                break;
            case 13: // sensors
                sensor();
                break;
            case 14: // orbit
                orbit();
                if (ididit) hitme = TRUE;
                break;
            case 15: // transport "beam"
                beam();
                break;
            case 16: // mine
                mine();
                if (ididit) hitme = TRUE;
                break;
            case 17: // crystals
                usecrystals();
                break;
            case 18: // shuttle
                shuttle();
                if (ididit) hitme = TRUE;
                break;
            case 19: // Planet list
                preport();
                break;
            case 20: // Status information
                srscan(2);
                break;
            case 21: // Game Report 
                report(0);
                break;
            case 22: // use COMPUTER!
                eta();
                break;
            case 23:
                listCommands(TRUE);
                break;
            case 24: // Emergency exit
                /* The panels are the game, so hiding the screen has to
                   take them with it -- clearscreen() only wipes the
                   message window. */
                tui_gameover();
                clearscreen(); // Hide screen
                freeze(TRUE); // forced save
                exit(1); // And quick exit
                break;
            case 25:
                probe(); // Launch probe
                break;
#ifdef CLOAKING
            case 26:
                cloak(); // turn on/off cloaking
                if (iscloaking) {
                    attack(2); // We will be seen while we cloak
                    iscloaking = FALSE;
                    if (damage[DCLOAK] == 0) // don't cloak if we got damaged while cloaking!
                        iscloaked = TRUE;
                }
                break;
#endif
#ifdef CAPTURE
            case 27:
                capture(); // Attempt to get Klingon ship to surrender
                if (ididit) hitme = TRUE;
                break;
#endif
#ifdef SCORE
            case 28:
                score(1); // get the score
                break;
#endif
            case 29: // Abandon Ship
                abandn();
                break;
            case 30: // Self Destruct
                dstrct();
                break;
            case 31: // Save Game
                freeze(FALSE);
                if (skill > SGOOD)
                    prout("WARNING--Frozen games produce no plaques!");
                break;
            case 32: // Try a desparation measure
                deathray();
                if (ididit) hitme = TRUE;
                break;
#ifdef DEBUG
            case 33: // What do we want for debug???
                debugme();
                break;
#endif
            case 34: // Call for help
                help();
                break;
            case 35:
                alldone = 1; // quit the game
                tui_gameover();
#ifdef DEBUG
                if (idebug) score(0);
#endif
                break;
            case 36:
                helpme(); // get help
                break;
        }
        for (;;) {
            if (alldone) break; // Game has ended
#ifdef DEBUG
            if (idebug) prout("2500");
#endif
            if (Time != 0.0) {
                events();
                if (alldone) break; // Events did us in
            }
            if (d.galaxy[quadx][quady] == 1000) { // Galaxy went Nova!
                atover(0);
                continue;
            }
            if (nenhere == 0) movetho();
            if (hitme && justin == 0) {
                attack(2);
                if (alldone) break;
                if (d.galaxy[quadx][quady] == 1000) { // went NOVA! 
                    atover(0);
                    hitme = TRUE;
                    continue;
                }
            }
            break;
        }
        if (alldone) break;
    }
}


/* The two lines that explain a refused full-screen display.
 *
 * Each turns on a different fact, and they are chosen separately
 * because both can be true at once. The first is about the size: is
 * the window itself under the 72x24 the panels need, or is it fine
 * and only what curses was told to believe that is wrong? The second
 * is about what the player can do, and the ordinary advice -- grow
 * to 72x24 and the next game gets panels -- cannot be relied on
 * once a pin has refused them. It is the size that line names that
 * fails: 72x24 exactly still loses to a pin under the floor, and
 * loses to one over the window unless the pin is 72 or 24 itself.
 * Growing is not useless, though -- COLUMNS=72 in a 70-column
 * window is taken at exactly 72x24, measured, and a bigger pin as
 * soon as the window catches up with it. It is not something to
 * promise, which is what the ordinary line does.
 *
 * Shared by the startup notice and the one the play-again retry
 * prints, which is the point: they described the same state two
 * different ways for as long as they each worded it themselves. The
 * retry differs in two places only -- it says what it measured,
 * since the player has just resized and the number is the thing they
 * cannot otherwise get, and it stays quiet where there is nothing to
 * add. Both gates rewrite all four refused* statics, so havesizes
 * reflects the ioctl inside this attempt's tui_init(), not an
 * earlier one. Where that ioctl gave nothing, a retry prints the
 * action line alone, or nothing at all if no pin is to blame --
 * it needs the ioctl to fail there having worked in
 * tui_size_changed_since_refusal() a moment before, which could
 * not be constructed on a tty.
 *
 * The game breaks these lines rather than the terminal: the floor is
 * LINES < 24 || COLS < 72, so height alone reaches it and a notice
 * prints on a 100-column screen as readily as a 40-column one. The
 * eight forms with no numbers in run 45 to 57 columns, four of them
 * at 57: the two plain lines and the two that name both variables.
 * At 50 the too-small line breaks mid-word, "classic d" /
 * "isplay.", which is the shape the budget exists to stop
 * spreading; the grow-it line breaks cleanly there.
 *
 * The two that carry numbers get no column count here. Five attempts
 * to state one were wrong, four of them caught in review: a bound has
 * to be computed over the shapes that reach the line, and the numbers
 * come from the terminal, which goes wider than anyone assumes -- tmux
 * alone will make a pane 10000 columns. What holds without a count,
 * computed over those shapes: the size report needs both terminal axes
 * past the floor, and comes out at least fifteen columns inside the
 * window it prints on, so it cannot wrap at all. Fifteen exactly, and
 * only just: at 72x10000 with COLUMNS pinned and LINES left free the
 * line is 57 columns in a 72-column window. That rests on ncurses
 * clamping an exported size to 512, measured, which is what holds the
 * pinned axis to three digits; the free one follows the window and can
 * be five. Unclamped the worst case is 69 in 72, so the conclusion
 * survives without it and the number does not. The retry line does
 * print on small windows, but under 72 columns it cannot pass 52,
 * inside the 57 the fixed lines already reach there. So neither wraps
 * anywhere the others do not, which is the whole of what the width
 * mattered for.
 *
 * A third line carries numbers, the grow-or-unset line added for #174,
 * and that one does get a count, because unlike the two above it can
 * be bounded: 61 columns, in a window of at least 72. Its fixed text
 * is 38, and the rest is a blame of 5, 7 or 17 plus the two numbers.
 * The 17 needs both axes named, and tui_refusal_growable() answers
 * FALSE where an axis is under the floor, so both are then oversize --
 * and an oversize axis is a pinned one, held to three digits by the
 * same 512 clamp. That is where this differs from the size report:
 * the conclusion rests on the clamp rather than surviving without it,
 * since two unclamped pinned numbers would take the line to 75.
 * Measured at both ends, on a 100x30 pane with COLUMNS=99999 and on a
 * 72x24 one with LINES and COLUMNS both 99999: curses reports 512
 * either way, and the second prints the 61-column line with eleven
 * columns to spare. The window is at least 72 because sst.c reaches
 * this branch only where smallwindow is false, which is both terminal
 * axes past the floor.
 *
 * The skip(1) after each proutf is not decoration: proutf does not
 * end its line, where prout is exactly that pair. #162. */
static void refusal_notice(int retry) {
	const char *blame = tui_refusal_blame();
	int cols, rows, termcols, termrows, needcols, needrows;
	int havesizes = tui_refused_sizes(&cols, &rows, &termcols, &termrows);
	int smallwindow = !havesizes || termrows < MINROWS ||
			  termcols < MINCOLS;

	if (!smallwindow) {
		proutf("Terminal is %dx%d but LINES/COLUMNS make it %dx%d.",
		       termcols, termrows, cols, rows);
		skip(1);
	}
	else if (retry && havesizes) {
		proutf("Terminal is %dx%d -- need 72x24, staying classic.",
		       termcols, termrows);
		skip(1);
	}
	else if (!retry)
		prout("Terminal too small (need 72x24) -- using classic display.");
	if (blame != NULL && smallwindow) {
		/* Both, because for a pin under the floor neither alone
		   gets the player there: unset it and the window is still
		   short, grow the window and the pin still refuses. Told
		   one at a time it takes two runs to find that out.
		   Not so for a pin merely bigger than the window, which
		   a big enough window accepts: COLUMNS=75 at 70x20 comes
		   here, and growing to 100x30 alone brings the panels up
		   at 75 columns, measured. The line asks that player for
		   one action more than they need rather than splitting
		   into a further two forms. */
		proutf("Grow to 72x24, unset %s, and rerun sst -t.", blame);
		skip(1);
	}
	else if (blame != NULL &&
		 tui_refusal_growable(&needcols, &needrows)) {
		/* The split the branch above declines to make, which here
		   is worth making: the window is over the floor, so growing
		   it really is an alternative to unsetting and not an extra
		   demand. Told to unset and nothing else, a player who had
		   done a working thing -- widened the window, and fallen
		   short -- was pointed away from it, with "classic for now"
		   reading as though no route were left in the session at
		   all. Measured for #174: COLUMNS=190 in a 100x30 pane
		   widened to 150x30, where 190 columns brings the panels up
		   and the line said to unset.
		   The rerun rides with the unset half alone, because only
		   that half needs one: the pin is in this process's
		   environment, where growing is answered at the next
		   play-again -- measured, COLUMNS=190 at 100x30 widened to
		   190x30 at that prompt brings the panels up in the same
		   process. Told "then rerun" a player who grew the window
		   was sent through a quit for nothing.
		   Two cuts here are deliberate, not oversights. The grow
		   half says nothing about when it takes effect, which the
		   no-pin line below does say ("and the next game gets
		   panels"). There is no room: "Grow to %dx%d for panels
		   next game, or unset %s and rerun sst -t." is 59
		   characters of fixed text, so 82 in the worst case
		   counted the way the comment above this function counts
		   the 61, and this line has to fit 72. The retry notice
		   closes that loop instead, for the player who grows and
		   misses.
		   And there is no comma before "and rerun", though the
		   compound branch above punctuates the same clause. Set
		   off by a comma the rerun reads as applying to the grow
		   half too, which is the misdirection this wording exists
		   to remove. */
		proutf("Grow to %dx%d, or unset %s and rerun sst -t.",
		       needcols, needrows, blame);
		skip(1);
	}
	else if (blame != NULL) {
		proutf("Unset %s, rerun sst -t -- classic for now.", blame);
		skip(1);
	}
	else if (!retry)
		prout("Grow the terminal to 72x24 and the next game gets panels.");
}


int main(int argc, char **argv) {
	int usetui = 0;
	int resized;

	while (argc > 1) { // look for -f and -t options
		if (strcmp(argv[1], "-f") == 0)
			coordfixed = 1;
		else if (strcmp(argv[1], "-t") == 0)
			usetui = 1;
		else
			break;
		argc--;
		argv++;
	}

	if (usetui && !tui_init()) {
		if (!stdio_is_terminal())
			prout("Full-screen mode needs a terminal -- using the classic display.");
		else if (!tui_terminal_capable())
			prout("This terminal cannot do full-screen mode -- using the classic display.");
		else
			refusal_notice(FALSE);
	}

	prelim();

	if (argc > 1) {
		fromcommandline = 1;
		line[0] = '\0';
		while (--argc > 0) {
			strcat(line, *(++argv));
			strcat(line, " ");
		}
	}
	else fromcommandline = 0;


	while (TRUE) { /* Play a game */
		setup();
		if (alldone) {
			/* A saved game can be a finished one: alldone rides
			   along in `a`, which freeze and thaw write and read
			   whole. The panels went up when it loaded, so take
			   them down before the epilogue prints. */
			tui_gameover();
			score(0);
			alldone = 0;
		}
		else makemoves();
		skip(2);
		stars();
		skip(1);

		if (tourn && alldone) {
			proutf("Do you want your score recorded?");
			if (ja()) {
				chew2();
				freeze(FALSE);
			}
		}
		proutf("Do you want to play again?");
		if (!ja()) break;
		/* The terminal may have grown since the choice was made
		   at startup, and a player told it was too small has
		   every reason to have made it bigger. Asked again per
		   game rather than per resize: mid-game is the one time
		   there is a game on screen to lose, and between games
		   there is nothing the panels can be wrong about --
		   tui_gameover() has already cleared tui_ingame.
		   Whether to say anything when it fails again turns on
		   what the player did. One who left the terminal alone
		   was told at startup and does not need telling every
		   game. One who grew it and missed -- 80x22, or "80x24"
		   inside tmux, where the status bar leaves the pane 23
		   rows -- did what the notice asked and has no way to
		   tell a mis-sized window from a broken promise. Asked
		   before the retry, which is what updates the size the
		   answer is measured against. #154.
		   Too small is not the only way to be refused, so this
		   is not only the player who grew a small window. An
		   exported LINES or COLUMNS can refuse a terminal of any
		   size, so a player who drags a 150-column window under
		   one arrives here having acted just as plainly, and is
		   owed the same answer. #169. */
		if (usetui && !tui_active) {
			resized = tui_size_changed_since_refusal();
			if (!tui_init() && resized)
				refusal_notice(TRUE);
		}
	}
	tui_shutdown();		/* so the farewell survives the screen restore */
	skip(1);
	prout("May the Great Bird of the Galaxy roost upon your home planet.");
	return 0;
}


void cramen(int i) {
	/* return an enemy */
	char *s;
	
	switch (i) {
		case IHR: s = "Romulan"; break;
		case IHK: s = "Klingon"; break;
		case IHC: s = "Commander"; break;
		case IHS: s = "Super-commander"; break;
		case IHSTAR: s = "Star"; break;
		case IHP: s = "Planet"; break;
		case IHB: s = "Starbase"; break;
		case IHBLANK: s = "Black hole"; break;
		case IHT: s = "Tholian"; break;
		case IHWEB: s = "Tholian web"; break;
		default: s = "Unknown??"; break;
	}
	proutn(s);
}

void cramlc(int key, int x, int y) {
	if (key == 1) proutn(" Quadrant");
	else if (key == 2) proutn(" Sector");
	proutn(" ");
	crami(x, 1);
	proutn(" - ");
	crami(y, 1);
}

void crmena(int i, int enemy, int key, int x, int y) {
	if (i == 1) proutn("***");
	cramen(enemy);
	proutn(" at");
	cramlc(key, x, y);
}

void crmshp(void) {
	char *s;
	switch (ship) {
		case IHE: s = "Enterprise"; break;
		case IHF: s = "Faerie Queene"; break;
		default:  s = "Ship???"; break;
	}
	proutn(s);
}

void stars(void) {
	prouts("******************************************************");
	skip(1);
}

double expran(double avrage) {
	return -avrage*log(1e-7 + Rand());
}

/* The game's random numbers, which are its own rather than the C
 * library's.
 *
 * sst.doc promises that a tournament number is all two players need to
 * share to play the same galaxy, and that identical actions in it lead
 * to identical results. rand() cannot keep that promise across a room:
 * glibc and the BSD libc that macOS uses produce entirely different
 * sequences from the same seed, so tournament 42 has never been the
 * same game on a Mac as on a Linux box.
 *
 * This is PCG-XSH-RR with a fixed increment (O'Neill, 2014): a 64-bit
 * step, a xorshift down to 32 bits and a rotate. Written in unsigned
 * arithmetic of stated width, so it computes the same sequence
 * everywhere a C17 compiler does.
 *
 * Seeding a game from the clock still gives an unpredictable one; what
 * changes is that a tournament number now means the same thing on
 * every machine. It also means a different galaxy from the one that
 * number used to give here -- unavoidable, and the number was never
 * portable to begin with. */
static uint64_t rngstate = 0x853c49e6748fea9bULL;

static uint32_t rngnext(void) {
	uint64_t old = rngstate;
	uint32_t xorshifted, rot;

	rngstate = old * 6364136223846793005ULL + 1442695040888963407ULL;
	xorshifted = (uint32_t)(((old >> 18) ^ old) >> 27);
	rot = (uint32_t)(old >> 59);
	return (xorshifted >> rot) | (xorshifted << ((32 - rot) & 31));
}

void sst_srand(unsigned int seed) {
	rngstate = seed;
	(void)rngnext();	/* let the seed spread before it is used */
}

void randomize(void) {
	sst_srand((unsigned int)time(NULL));
}

double Rand(void) {
	/* 2^32 exactly, so the result lands in [0,1) with no rounding
	   surprises and the same bits on every machine. */
	return rngnext() / 4294967296.0;
}

void iran8(int *i, int *j) {
	*i = Rand()*8.0 + 1.0;
	*j = Rand()*8.0 + 1.0;
}

void iran10(int *i, int *j) {
	*i = Rand()*10.0 + 1.0;
	*j = Rand()*10.0 + 1.0;
}

void chew(void) {
	linecount = 0;
	linep = line;
	*linep = 0;
}

void chew2(void) {
	/* return IHEOL next time */
	linecount = 0;
	linep = line+1;
	*linep = 0;
}

/* Read one line of whatever the player is being asked for.
 *
 * The input stream ending means no answer is ever coming, so it ends
 * the session the same way quitting does. Without that the game keeps
 * asking a question nobody is left to answer, filling the terminal --
 * or a CI runner's disk -- with prompts. */
void readinput(char *buf, int buflen) {
	int got;

	prompted = TRUE;

	if (tui_active)
		got = tui_readline(buf, buflen);
	else {
		/* fgets says NULL for a read that was interrupted as well
		   as for one that reached the end of the input, and only
		   the second means no answer is ever coming. Telling them
		   apart is not hypothetical here: tui_init() runs initscr()
		   before it discovers the terminal is too small for the
		   panels, and endwin() did not take back the SIGWINCH
		   handler initscr() installed -- so a game that had fallen
		   back to the classic display had its reads interrupted by
		   a resize, and this ended the game mid-question. That is
		   what a player got for doing the obvious thing about
		   "Terminal too small (need 72x24)". Issue #150.
		   Past tense since #152, which restores that disposition on
		   the give-up path, so no resize reaches here now. Nothing
		   else does either. Read off SigCgt in /proc/<pid>/status and
		   off sa_flags after an initscr()/endwin() pair, against
		   ncurses 6.6: the fallback game then catches SIGINT and
		   SIGTERM and nothing more, and SIGCONT is not caught at
		   all -- so neither a Ctrl-Z nor a stop-and-continue makes
		   this read return.
		   SIGTSTP was on that list, and the reason given here was
		   that curses' handler carried SA_RESTART. #158 put the
		   disposition back on the same give-up path, so there is
		   no curses handler to carry anything and a default stop
		   and continue restarts the read. The conclusion is the
		   one it always was; the premise changed. osx.c's getch()
		   carries the same correction for the keystroke reader.
		   The retry stays anyway. Treating an interrupted read as
		   the end of the input is wrong whatever the signals happen
		   to be, and the next handler installed anywhere in the
		   classic path would make it wrong in the way #150 was.
		   feof() is the question worth asking, and errno == EINTR
		   the answer worth retrying. Anything else -- a real read
		   error -- still ends the session, as it did.
		   One limit, left alone: glibc returns NULL having already
		   copied part of a line, and the retry then starts again
		   at buf[0] with those bytes gone. Canonical mode delivers
		   a line whole or not at all, so reaching it takes
		   something like an end-of-file mid-line and a resize on
		   top of it -- and the old code ended the session there,
		   so it is not a regression either way. */
		errno = 0;
		while ((got = fgets(buf, buflen, stdin) != NULL) == 0
		       && !feof(stdin) && errno == EINTR) {
			clearerr(stdin);
			errno = 0;
		}
	}
	if (got) {
		/* Hand the line back with nothing trailing. \r as well as
		   \n: a script written on Windows would otherwise have
		   every command rejected, and the carriage return behind
		   that does not show up in the error the player is given. */
		char *end = buf + strlen(buf);
		while (end > buf && (end[-1] == '\n' || end[-1] == '\r'))
			*--end = '\0';
		return;
	}

	tui_shutdown();
	linecount = 0;		/* don't page on the way out */
	skip(1);
	/* Say which ending this is. A player scripting the game can tell
	   the script ran out from the game finishing, and the journey test
	   relies on it to catch a script that stops short. */
	prout("[Transmission ends.]");
	skip(2);
	stars();		/* the sign-off the quit path prints too */
	skip(1);
	prout("May the Great Bird of the Galaxy roost upon your home planet.");
	exit(0);
}

int scan(void) {
	int i;
	char *cp;

	linecount = 0;

	// Init result
	aaitem = 0.0;
	*citem = 0;

	// Read a line if nothing here
	if (*linep == 0) {
		if (linep != line) {
			chew();
			return IHEOL;
		}
		readinput(line, sizeof(line));
		linep = line;
	}
	// Skip leading white space
	while (*linep == ' ') linep++;
	// Nothing left
	if (*linep == 0) {
		chew();
		return IHEOL;
	}
	if (isdigit(*linep) || *linep=='+' || *linep=='-' || *linep=='.') {
		// treat as a number
	    if (sscanf(linep, "%lf%n", &aaitem, &i) < 1) {
		linep = line; // Invalid numbers are ignored
		*linep = 0;
		return IHEOL;
	    }
	    else {
		// skip to end
		linep += i;
		return IHREAL;
	    }
	}
	// Treat as alpha
	cp = citem;
	while (*linep && *linep!=' ') {
		if ((cp - citem) < 9) *cp++ = tolower(*linep);
		linep++;
	}
	*cp = 0;
	return IHALPHA;
}

int ja(void) {
	chew();
	while (TRUE) {
		scan();
		chew();
		if (*citem == 'y') return TRUE;
		if (*citem == 'n') return FALSE;
		proutn("Please answer with \"Y\" or \"N\":");
	}
}

void cramf(double x, int w, int d) {
	char buf[64];
	sprintf(buf, "%*.*f", w, d, x);
	proutn(buf);
}

void crami(int i, int w) {
	char buf[16];
	sprintf(buf, "%*d", w, i);
	proutn(buf);
}

double square(double i) { return i*i; }
									
static void clearscreen(void) {
	/* Somehow we need to clear the screen */
	if (tui_active)
		tui_clearmsg();
	else
		proutn("\033[2J\033[0;0H");	/* Hope for an ANSI display */
}

/* We will pull these out in case we want to do something special later */

void pause(int i) {
#ifdef CLOAKING
	if (iscloaked) return;
#endif
	proutn("\n");
	if (i==1) {
		if (skill > SFAIR)
			prout("[ANNOUNCEMENT ARRIVING...]");
		else
			prout("[IMPORTANT ANNOUNCEMENT ARRIVING -- HIT SPACE BAR TO CONTINUE]");
		getch();
	}
	else {
		if (skill > SFAIR)
			proutn("[CONTINUE?]");
		else
			proutn("[HIT SPACE BAR TO CONTINUE]");
		getch();
		proutn("\r                           \r");
	}
	if (i != 0) {
		clearscreen();
	}
    linecount = 0;
}


void skip(int i) {
	while (i-- > 0) {
		linecount++;
		/* Paging holds output back so it cannot scroll away unread.
		   Until the game has asked for a line there is nobody to hold
		   it for, and the keystroke such a pause waits for is the
		   first one the player will type, which it eats. That is not
		   a corner: the startup banner overflows the message window
		   on a 24-line terminal, and setup answers given on the
		   command line run straight into the briefing, so it was the
		   ordinary first move in -t. Letting that text scroll instead
		   is what any terminal too small to hold it would do anyway.

		   The gate is deliberately not conditional on tui_active: the
		   reasoning holds in either display. It changes nothing in the
		   plain one only because scan() zeroes linecount often enough
		   that nothing printed before the first prompt comes near 23
		   lines -- worth rechecking if the preamble ever grows. */
		if (prompted && linecount >= (tui_active ? tui_pageheight() : 23))
			pause(0);
		else if (tui_active)
			tui_puts("\n");
		else
			putchar('\n');
	}
}


void proutn(char *s) {
	if (tui_active)
		tui_puts(s);
	else
		fputs(s, stdout);
}

void prout(char *s) {
	proutn(s);
	skip(1);
}

void proutf(const char *fmt, ...) {
	char buf[512];
	char *p, *nl;
	va_list ap;
	va_start(ap, fmt);
	vsnprintf(buf, sizeof(buf), fmt, ap);
	va_end(ap);
	/* A line at a time, so the newlines written inside the string
	   count towards paging like any others. Handing the whole
	   thing to proutn() moves the screen on without moving the
	   pager, which is how the briefing -- the one place the game
	   states the mission and the deadline -- came to scroll away
	   unread in a ten-line message window. */
	p = buf;
	while ((nl = strchr(p, '\n')) != NULL) {
		*nl = '\0';
		proutn(p);
		skip(1);
		p = nl + 1;
	}
	if (*p != '\0') proutn(p);
}

void prouts(char *s) {
	clock_t endTime;
	if (tui_active) {
		tui_puts_slow(s);
		return;
	}
	/* print slowly! */
	while (*s) {
		endTime = clock() + CLOCKS_PER_SEC*0.05;
		while (clock() < endTime) ;
		putchar(*s++);
		fflush(stdout);
	}
}

void huh(void) {
	chew();
	skip(1);
	prout("Beg your pardon, Captain?");
}

int isit(char *s) {
	/* New function -- compares s to scaned citem and returns true if it
	   matches to the length of s */

	return strncmp(s, citem, max(1, strlen(citem))) == 0;

}

#ifdef DEBUG
void debugme(void) {
	proutn("Reset levels? ");
	if (ja() != 0) {
		if (energy < inenrg) energy = inenrg;
		shield = inshld;
		torps = intorps;
		lsupres = inlsr;
	}
	proutn("Reset damage? ");
	if (ja() != 0) {
		int i;
		for (i=0; i <= ndevice; i++) if (damage[i] > 0.0) damage[i] = 0.0;
		stdamtim = 1e30;
	}
	proutn("Toggle idebug? ");
	if (ja() != 0) {
		idebug = !idebug;
		if (idebug) prout("Debug output ON");
		else prout("Debug output OFF");
	}
	proutn("Cause selective damage? ");
	if (ja() != 0) {
		int i, key;
		for (i=1; i <= ndevice; i++) {
			proutn("Kill ");
			proutn(device[i]);
			proutn("? ");
			chew();
			key = scan();
			if (key == IHALPHA &&  isit("y")) {
				damage[i] = 10.0;
				if (i == DRADIO) stdamtim = d.date;
			}
		}
	}
	proutn("Examine/change events? ");
	if (ja() != 0) {
		int i;
		for (i = 1; i < NEVENTS; i++) {
			int key;
			if (future[i] == 1e30) continue;
			switch (i) {
				case FSNOVA:  proutn("Supernova       "); break;
				case FTBEAM:  proutn("T Beam          "); break;
				case FSNAP:   proutn("Snapshot        "); break;
				case FBATTAK: proutn("Base Attack     "); break;
				case FCDBAS:  proutn("Base Destroy    "); break;
				case FSCMOVE: proutn("SC Move         "); break;
				case FSCDBAS: proutn("SC Base Destroy "); break;
			}
			cramf(future[i]-d.date, 8, 2);
			chew();
			proutn("  ?");
			key = scan();
			if (key == IHREAL) {
				future[i] = d.date + aaitem;
			}
		}
		chew();
	}
	proutn("Make universe visible? ");
	if (ja() != 0) {
		int i, j;
		for (i = 1; i < 9; i++) 
		{
			for (j = 1; j < 9; j++)
			{
				starch[i][j] = 1;
			}
		}
	}
}
			

#endif
