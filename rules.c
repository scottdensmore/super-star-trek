#include "sst.h"
#include "rules.h"

/* Rules the manual states in numbers, lifted out of the prose that
 * narrates them so that a test can ask them a question.
 *
 * The pattern is tuifmt.c's: the arithmetic here, the telling of it
 * where it was. Nothing in this file prints, reads, or touches curses.
 */

void score_compute(struct scoring *s, int inGame, double elapsed)
{
	/* "500 times your average Klingon ship/stardate kill rate. If you
	   lose the game, your kill rate is based on a minimum of 5
	   stardates." -- sst.doc:1363-1365.

	   Two conditions, doing different jobs. d.remkl != 0 is the
	   manual's "if you lose the game": Klingons left means the game
	   was not won. elapsed == 0 is not in the manual at all -- it
	   keeps the division below away from zero when a game ends on the
	   stardate it began. The rounding is the code's too; the manual
	   says only "500 times". */
	if ((elapsed == 0 || d.remkl != 0) && elapsed < 5.0) elapsed = 5.0;
	s->perdate = (d.killc + d.killk + d.nsckill)/elapsed;
	s->killrate = 500*s->perdate + 0.5;

	/* "You get a bonus if you win the game, based on your rating:
	   Novice=100 ... Emeritus=500." -- sst.doc:1366-1367. */
	s->winbonus = gamewon ? 100*skill : 0;

	/* "100 points for each starship you lose" -- sst.doc:1373. Flying
	   the Faerie Queene means the Enterprise is gone; flying neither
	   means both are. */
	if (ship == IHE) s->shipslost = 0;
	else if (ship == IHF) s->shipslost = 1;
	else s->shipslost = 2;

	/* "1 point for each Romulan ship surrendered" -- sst.doc:1362.
	   Nobody surrenders to a captain who has not won, and a game
	   still in progress has not. */
	s->surrendered = (gamewon == 0 || inGame) ? 0 : d.nromrem;

	s->total = 10*d.killk		/* sst.doc:1357 */
		 + 50*d.killc		/* sst.doc:1358 */
		 + 200*d.nsckill	/* sst.doc:1359 */
		 + 20*d.nromkl		/* sst.doc:1361 */
		 + s->surrendered	/* sst.doc:1362 */
		 + s->killrate		/* sst.doc:1363 */
		 + s->winbonus		/* sst.doc:1366 */
		 - 100*d.basekl		/* sst.doc:1372 */
		 - 100*s->shipslost	/* sst.doc:1373 */
		 - 45*nhelp		/* sst.doc:1375 */
		 - 10*d.nplankl		/* sst.doc:1376 */
		 - 5*d.starkl		/* sst.doc:1377 */
		 - casual;		/* sst.doc:1378 */
#ifdef CLOAKING
	s->total -= 100*ncviol;		/* sst.doc:1374 */
#endif
#ifdef CAPTURE
	s->total += 3*kcaptured;	/* sst.doc:1360 */
#endif
	if (alive == 0) s->total -= 200;	/* sst.doc:1371 */
}
