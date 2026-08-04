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

void promotion_compute(struct promotion *p, double elapsed)
{
	/* "if you have lost 100 or more points in penalties, the required
	   kill rate goes up" -- sst.doc:1383-1385. Below a hundred the
	   game rounds the whole thing away rather than scaling it.

	   Which penalties count is where the code and the manual part
	   company. The manual says "points in penalties", and the score
	   sheet lists eight kinds; this counts six of them. Losing your
	   ship is folded in below. The death penalty is moot -- a dead
	   captain is not promoted -- but a Treaty of Algeron violation
	   costs a hundred points and does not raise this bar at all. See
	   the note in tests/test_rules.c. */
	p->penalties = 5.0*d.starkl		/* sst.doc:1377 */
		     + casual			/* sst.doc:1378 */
		     + 10.0*d.nplankl		/* sst.doc:1376 */
		     + 45.0*nhelp		/* sst.doc:1375 */
		     + 100.0*d.basekl;		/* sst.doc:1372 */
	if (ship == IHF) p->penalties += 100.0;		/* sst.doc:1373 */
	else if (ship == 0) p->penalties += 200.0;
	if (p->penalties < 100.0) p->penalties = 0.0;

	/* "Normally, the required kill rate is 0.1 * skill * (skill + 1.0)
	   + 0.1, where skill ranges from 1 for Novice to 5 for Emeritus."
	   -- sst.doc:1385-1387. The penalty term is the "goes up" part;
	   its size is the code's own, the manual only says it rises. */
	p->needed = 0.1*skill*(skill + 1.0) + 0.1 + 0.008*p->penalties;

	/* A game won in under five stardates is promoted without being
	   asked for a rate -- which the manual does not mention, and
	   which the division below could not survive anyway when a game
	   is won on the stardate it began. */
	p->achieved = elapsed < 5.0 ? 0.0
		    : (d.killk + d.killc + d.nsckill)/elapsed;
	p->earned = elapsed < 5.0 || p->achieved >= p->needed;
}
