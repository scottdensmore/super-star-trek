/* The game's rules, checked against sst.doc.
 *
 * Every expected number below was read out of the manual, not out of
 * the program. That is the whole point of this file: a test whose
 * expectation came from running the code agrees with the code by
 * construction, and tests/golden.sh already records what the code does
 * far more cheaply. This one is a second opinion, so that when the two
 * disagree somebody has to decide which is wrong.
 *
 * Line references are to sst.doc. If one of these fails, read the
 * manual before changing the test.
 *
 * Defines INCLUDED so sst.h instantiates the game-state globals here.
 */
#define INCLUDED
#include "../sst.h"
#include "../rules.h"

static int failures = 0;

/* Rates and penalties are doubles the manual states exactly, so an
 * exact comparison would be right in principle -- but 0.1*2*3 + 0.1 is
 * not 0.7 in binary. A millionth is far tighter than any rule here. */
static void checkrate(const char *what, double got, double want) {
	/* Inverted on purpose: written the other way round, a NaN
	   compares false against everything and slips through. */
	if (!(got - want <= 1e-6 && want - got <= 1e-6)) {
		failures++;
		printf("FAIL %s\n  want: %g\n  got:  %g\n", what, want, got);
	}
}

/* For a number the manual gives only approximately. */
static void checkclose(const char *what, double got, double want, double tol) {
	if (!(got - want <= tol && want - got <= tol)) {
		failures++;
		printf("FAIL %s\n  want: %g (within %g)\n  got:  %g\n",
		       what, want, tol, got);
	}
}

static void checkint(const char *what, int got, int want) {
	if (got != want) {
		failures++;
		printf("FAIL %s\n  want: %d\n  got:  %d\n", what, want, got);
	}
}

/* A game in which nothing whatever has happened: nobody killed,
 * nothing lost, the Enterprise intact and the captain alive. Every
 * test below starts here and turns on one rule at a time, so what it
 * is measuring is that rule and not the state around it.
 *
 * How long the game ran is not set here -- it is an argument to
 * score_compute, so each test says it itself. */
static void nothing_happened(void) {
	memset(&d, 0, sizeof(d));
	d.remkl = 0;	/* the game is over: nothing left to kill */
	skill = SNOVICE;
	gamewon = 0;
	alive = 1;
	ship = IHE;
	casual = nhelp = 0;
#ifdef CLOAKING
	ncviol = 0;
#endif
#ifdef CAPTURE
	kcaptured = 0;
#endif
}

/* What one rule is worth on its own.
 *
 * Over a very long game, so that rule (7) -- 500 times the kills per
 * stardate -- rounds to nothing and the number returned is only the
 * rule under test. Writing 10 stardates here instead cost an hour:
 * three Klingons scored 180 rather than 30, because the kills pay
 * twice, once as kills and once as a rate. The manual says so; the
 * first draft of this file had simply forgotten to read that far. */
static int scored(void) {
	struct scoring s;
	score_compute(&s, FALSE, 100000.0);
	/* Asserted, not assumed: a later test with a hundred kills in it
	   would drag the rate back above zero and every number below
	   would be quietly wrong. */
	checkint("the long game keeps the kill rate out of it", s.killrate, 0);
	return s.total;
}

/* "You gain -- 10 points for each ordinary Klingon ship you destroy,
 * 50 for each commander, 200 for the Super-Commander, 3 for each
 * Klingon captured, 20 for each Romulan destroyed, 1 for each Romulan
 * surrendered." -- sst.doc:1357-1362 */
static void test_score_gains(void) {
	nothing_happened();
	checkint("a game where nothing happened scores nothing", scored(), 0);

	nothing_happened();
	d.killk = 3;
	checkint("three ordinary Klingons", scored(), 30);

	nothing_happened();
	d.killc = 2;
	checkint("two commanders", scored(), 100);

	nothing_happened();
	d.nsckill = 1;
	checkint("the Super-Commander", scored(), 200);

	nothing_happened();
	d.nromkl = 4;
	checkint("four Romulans destroyed", scored(), 80);

#ifdef CAPTURE
	nothing_happened();
	kcaptured = 7;
	checkint("seven Klingons captured", scored(), 21);
#endif
}

/* "1 point for each Romulan ship surrendered" -- sst.doc:1362 -- and
 * nobody surrenders to a captain who has not won. */
static void test_score_surrender(void) {
	struct scoring s;

	nothing_happened();
	d.nromrem = 6;
	score_compute(&s, FALSE, 10.0);
	checkint("surrender counts for nothing without a win", s.surrendered, 0);

	nothing_happened();
	d.nromrem = 6;
	gamewon = 1;
	score_compute(&s, FALSE, 10.0);
	checkint("six Romulans surrender to a winner", s.surrendered, 6);
	/* And are worth a point each on the sheet, alongside the 100 that
	   winning at novice brings with it. */
	checkint("a point for each of them", s.total, 6 + 100);

	nothing_happened();
	d.nromrem = 6;
	gamewon = 1;
	score_compute(&s, TRUE, 10.0);
	checkint("nobody has surrendered while the game is still on",
		 s.surrendered, 0);
}

/* "You lose -- 200 points if you get yourself killed, 100 for each
 * starbase you destroy, 100 for each starship you lose, 100 for each
 * violation of the Treaty of Algeron, 45 for each call for help, 10
 * for each planet, 5 for each star, 1 for each casualty."
 * -- sst.doc:1371-1378 */
static void test_score_losses(void) {
	nothing_happened();
	alive = 0;
	checkint("getting yourself killed", scored(), -200);

	nothing_happened();
	d.basekl = 2;
	checkint("two starbases destroyed", scored(), -200);

	nothing_happened();
	nhelp = 3;
	checkint("three calls for help", scored(), -135);

	nothing_happened();
	d.nplankl = 4;
	checkint("four planets destroyed", scored(), -40);

	nothing_happened();
	d.starkl = 6;
	checkint("six stars destroyed", scored(), -30);

	nothing_happened();
	casual = 17;
	checkint("seventeen casualties", scored(), -17);

#ifdef CLOAKING
	nothing_happened();
	ncviol = 2;
	checkint("two Algeron violations", scored(), -200);
#endif
}

/* "100 points for each starship you lose" -- sst.doc:1373. Which ship
 * you are flying says how many have gone. */
static void test_score_ships_lost(void) {
	struct scoring s;

	nothing_happened();
	ship = IHE;
	score_compute(&s, FALSE, 10.0);
	checkint("still in the Enterprise", s.shipslost, 0);
	checkint("and so nothing lost", s.total, 0);

	nothing_happened();
	ship = IHF;
	score_compute(&s, FALSE, 10.0);
	checkint("down to the Faerie Queene", s.shipslost, 1);
	checkint("one starship lost", s.total, -100);

	nothing_happened();
	ship = 0;
	score_compute(&s, FALSE, 10.0);
	checkint("no ship at all", s.shipslost, 2);
	checkint("two starships lost", s.total, -200);
}

/* "500 times your average Klingon ship/stardate kill rate. If you lose
 * the game, your kill rate is based on a minimum of 5 stardates."
 * -- sst.doc:1363-1365 */
static void test_score_kill_rate(void) {
	struct scoring s;

	/* Ten Klingons in ten stardates is one a stardate, which is 500. */
	nothing_happened();
	d.killk = 10;
	score_compute(&s, FALSE, 10.0);
	checkint("one kill a stardate", s.killrate, 500);
	checkint("and the kills themselves on top", s.total, 500 + 100);

	/* Commanders and the Super-Commander count towards the rate as
	   Klingon ships, which is what "Klingon ship/stardate" means. */
	nothing_happened();
	d.killc = 5;
	d.nsckill = 5;
	score_compute(&s, FALSE, 10.0);
	checkint("commanders count towards the rate", s.killrate, 500);

	/* The floor: two kills in one stardate is not two a stardate, it
	   is two in five. */
	nothing_happened();
	d.killk = 2;
	d.remkl = 1;		/* Klingons left, so the game was not won */
	score_compute(&s, FALSE, 1.0);
	checkint("a short game is scored over five stardates", s.killrate, 200);

	/* "If you *lose* the game" -- so a win in the same one stardate
	   keeps the rate it actually earned. This is the half of the rule
	   a recording of current behaviour could never tell you about. */
	nothing_happened();
	d.killk = 2;
	d.remkl = 0;
	gamewon = 1;
	score_compute(&s, FALSE, 1.0);
	checkint("a short win keeps its real rate", s.killrate, 1000);

	/* 500 times a third of a kill a stardate is 166.67, and the sheet
	   shows 167: the rounding is the code's, not the manual's. */
	nothing_happened();
	d.killk = 1;
	score_compute(&s, FALSE, 3.0);
	checkint("the rate is rounded rather than truncated", s.killrate, 167);
}

/* "You get a bonus if you win the game, based on your rating:
 * Novice=100, Fair=200, Good=300, Expert=400, Emeritus=500."
 * -- sst.doc:1366-1367 */
static void test_score_win_bonus(void) {
	struct scoring s;
	int grade[] = { SNOVICE, SFAIR, SGOOD, SEXPERT, SEMERITUS };
	int want[] = { 100, 200, 300, 400, 500 };
	int i;

	nothing_happened();
	skill = SEMERITUS;
	score_compute(&s, FALSE, 10.0);
	checkint("no bonus without a win", s.winbonus, 0);

	for (i = 0; i < 5; i++) {
		nothing_happened();
		gamewon = 1;
		skill = grade[i];
		score_compute(&s, FALSE, 10.0);
		checkint("bonus for winning at this rating", s.winbonus, want[i]);
		/* Nothing else happened in this game, so the bonus is the
		   whole of the score -- which is how we know it reached it. */
		checkint("and the bonus is the score", s.total, want[i]);
	}
}

/* The rules are a sum, so a game that did a bit of everything should
 * come to the total of its parts and nothing else.
 *
 * The manual numbers its list 1-15 but uses (8) twice -- once for the
 * win bonus and again for the death penalty -- so the citations above
 * are line numbers rather than rule numbers. */
static void test_score_adds_up(void) {
	struct scoring s;

	nothing_happened();
	d.killk = 4;		/*  +40 */
	d.killc = 1;		/*  +50 */
	d.nromkl = 2;		/*  +40 */
	d.basekl = 1;		/* -100 */
	nhelp = 2;		/*  -90 */
	d.starkl = 3;		/*  -15 */
	casual = 25;		/*  -25 */
	/* Ten stardates this time, not the long game the tests above use,
	   so the kill rate is part of the sum: five Klingon ships in ten
	   stardates is half a ship a stardate, and 500 times that is
	   +250. */
	score_compute(&s, FALSE, 10.0);
	checkint("a mixed game adds up", s.total,
		 40 + 50 + 40 - 100 - 90 - 15 - 25 + 250);
}

/* "Normally, the required kill rate is 0.1 * skill * (skill + 1.0) +
 * 0.1, where skill ranges from 1 for Novice to 5 for Emeritus."
 * -- sst.doc:1385-1387 */
static void test_promotion_threshold(void) {
	struct promotion p;
	int grade[] = { SNOVICE, SFAIR, SGOOD, SEXPERT, SEMERITUS };
	/* 0.1*s*(s+1) + 0.1, worked out from the manual by hand rather
	   than by running the game: 0.3, 0.7, 1.3, 2.1, 3.1. */
	double want[] = { 0.3, 0.7, 1.3, 2.1, 3.1 };
	int i;

	for (i = 0; i < 5; i++) {
		nothing_happened();
		skill = grade[i];
		promotion_compute(&p, 10.0);
		checkrate("the rate this rating asks for", p.needed, want[i]);
	}
}

/* "You may also be promoted one grade in rank if you play well enough.
 * Promotion is based primarily on your Klingon/stardate kill rate."
 * -- sst.doc:1380-1382 */
static void test_promotion_is_earned_by_the_rate(void) {
	struct promotion p;

	/* Novice asks 0.3 a stardate. Two kills in ten is 0.2. */
	nothing_happened();
	skill = SNOVICE;
	d.killk = 2;
	promotion_compute(&p, 10.0);
	checkrate("two kills in ten stardates", p.achieved, 0.2);
	checkint("and that is short of what Novice asks", p.earned, 0);

	/* Four in ten is 0.4, comfortably past it. */
	nothing_happened();
	skill = SNOVICE;
	d.killk = 4;
	promotion_compute(&p, 10.0);
	checkint("four in ten earns it", p.earned, 1);

	/* Commanders and the Super-Commander are Klingon ships too. */
	nothing_happened();
	skill = SNOVICE;
	d.killc = 2;
	d.nsckill = 2;
	promotion_compute(&p, 10.0);
	checkint("commanders count towards the rate", p.earned, 1);
	/* Two and two rather than three and one so that dropping either
	   term leaves 0.2 against a bar of 0.3, which is a difference
	   nobody has to squint at. */

	/* Nothing here sits on the boundary on purpose. Exactly three
	   kills in ten stardates ought to meet a bar of 0.3 and does not:
	   0.1*1*2 + 0.1 comes to 0.30000000000000004 in binary while 3/10
	   comes to slightly under, so the comparison goes the other way.
	   The manual says what the bar is, not what happens to a captain
	   who lands precisely on it, so neither does this. */
}

/* "if you have lost 100 or more points in penalties, the required kill
 * rate goes up" -- sst.doc:1383-1385 */
static void test_promotion_penalties(void) {
	struct promotion p;

	/* Ninety-nine points of penalty is under the hundred the manual
	   names, so the bar does not move: two calls for help (90) and
	   nine casualties (9). */
	nothing_happened();
	skill = SNOVICE;
	nhelp = 2;
	casual = 9;
	promotion_compute(&p, 10.0);
	checkrate("under a hundred points, nothing counts", p.penalties, 0.0);
	checkrate("so the bar is where it was", p.needed, 0.3);

	/* Above the floor the points are counted at the rates the score
	   sheet uses, so each one has to be worth what it says: five a
	   star, one a casualty, ten a planet, forty-five a call for help.
	   Chosen to clear a hundred between them and not otherwise. */
	nothing_happened();
	skill = SNOVICE;
	d.starkl = 4;		/*  20 */
	casual = 10;		/*  10 */
	d.nplankl = 5;		/*  50 */
	nhelp = 1;		/*  45 */
	promotion_compute(&p, 10.0);
	checkrate("each penalty counted at its own rate", p.penalties, 125.0);

	/* And calls for help are worth forty-five apiece on their own. */
	nothing_happened();
	skill = SNOVICE;
	nhelp = 3;
	promotion_compute(&p, 10.0);
	checkrate("three calls for help", p.penalties, 135.0);

	/* A destroyed starbase is a hundred on its own. */
	nothing_happened();
	skill = SNOVICE;
	d.basekl = 1;
	promotion_compute(&p, 10.0);
	checkrate("a starbase is a hundred", p.penalties, 100.0);
	checkint("which raises the bar", p.needed > 0.3, 1);
	/* By how much is the code's own: the manual says the rate "goes
	   up" and stops there. Recorded rather than derived, so that the
	   scale cannot drift unnoticed -- 0.3 plus 0.008 a point. */
	checkrate("and by 0.008 a point, which sst.doc does not state",
		  p.needed, 1.1);

	/* And a rate that would have earned promotion no longer does. */
	nothing_happened();
	skill = SNOVICE;
	d.killk = 4;
	promotion_compute(&p, 10.0);
	checkint("four kills earns it before a starbase is lost", p.earned, 1);
	d.basekl = 1;
	promotion_compute(&p, 10.0);
	checkint("but not after destroying a starbase", p.earned, 0);

#ifdef CLOAKING
	/* "100 points for each violation of the Treaty of Algeron"
	   -- sst.doc:1374 -- is a penalty like any other, and raises this
	   bar like any other. It did not, until #53. */
	nothing_happened();
	skill = SNOVICE;
	ncviol = 1;
	promotion_compute(&p, 10.0);
	checkrate("an Algeron violation is a hundred", p.penalties, 100.0);

	nothing_happened();
	skill = SNOVICE;
	d.killk = 4;
	promotion_compute(&p, 10.0);
	checkint("four kills earns it before any violation", p.earned, 1);
	ncviol = 1;
	promotion_compute(&p, 10.0);
	checkint("but not after cloaking in the Neutral Zone", p.earned, 0);

	/* And it is counted with the rest rather than after them: ninety-
	   nine points of other penalty is under the floor on its own, and
	   a violation carries the whole lot over it. Counted afterwards
	   the ninety-nine would vanish and the bar would come out at 100
	   rather than 199. */
	nothing_happened();
	skill = SNOVICE;
	nhelp = 2;		/*  90 */
	casual = 9;		/*   9 */
	ncviol = 1;		/* 100 */
	promotion_compute(&p, 10.0);
	checkrate("a violation carries the rest over the floor",
		  p.penalties, 199.0);
#endif
}

/* "100 points for each starship you lose" -- sst.doc:1373 -- counts
 * towards the penalties as much as anything else does. */
static void test_promotion_counts_lost_ships(void) {
	struct promotion p;

	nothing_happened();
	skill = SNOVICE;
	ship = IHF;
	promotion_compute(&p, 10.0);
	checkrate("the Enterprise lost is a hundred", p.penalties, 100.0);

	nothing_happened();
	skill = SNOVICE;
	ship = 0;
	promotion_compute(&p, 10.0);
	checkrate("both ships lost is two hundred", p.penalties, 200.0);
}

/* Not in the manual: a game that ends inside five stardates is
 * promoted without being asked for a rate at all. Recorded here as a
 * characterization, because sst.doc says nothing about it and the
 * division could not be done anyway on a game won the day it began. */
static void test_promotion_short_game(void) {
	struct promotion p;

	nothing_happened();
	skill = SEMERITUS;	/* the hardest bar there is */
	promotion_compute(&p, 4.9);
	checkint("a game won inside five stardates is promoted", p.earned, 1);
	checkrate("and is not asked for a rate", p.achieved, 0.0);

	nothing_happened();
	skill = SEMERITUS;
	promotion_compute(&p, 0.0);
	checkint("even one won on the day it began", p.earned, 1);
	/* And the division that would have been nought over nought never
	   happens: a NaN here would sail through a careless comparison. */
	checkrate("with no rate to divide", p.achieved, 0.0);
}

/* "Warp drive requires (distance)*(warp factor cubed) units of energy"
 * -- sst.doc:1467, distances in quadrants per sst.doc:1456. The WARP
 * FACTOR section says the same thing as a ratio: warp 10 "uses 1000
 * times as much energy" as warp 1 -- sst.doc:598-599.
 *
 * Both are worth asserting. The ratios say the exponent is three; only
 * the closed form says what a jump actually costs, and without it a
 * change that doubled the price of every warp jump in the game would
 * pass this file unremarked. */
static void test_warp_costs_the_cube_of_the_factor(void) {
	double one = warp_energy(1.0, 1.0, 0);
	double ten = warp_energy(1.0, 10.0, 0);

	checkrate("a quadrant at warp 1 costs one unit", one, 1.0);
	checkrate("two quadrants at warp 3 cost fifty-four",
		  warp_energy(2.0, 3.0, 0), 54.0);
	checkrate("warp 10 costs a thousand times warp 1", ten, one*1000.0);
	/* And in between, since a ratio at one point does not pin a curve:
	   warp 2 is eight times warp 1, not four and not sixteen. */
	checkrate("warp 2 costs eight times warp 1",
		  warp_energy(1.0, 2.0, 0), one*8.0);
	/* Distance is simply a multiplier. */
	checkrate("twice as far costs twice as much",
		  warp_energy(2.0, 3.0, 0), warp_energy(1.0, 3.0, 0)*2.0);
}

/* "You may move with your shields up, but this doubles the energy
 * required." -- sst.doc:579, and again at 648. */
static void test_warp_with_shields_up_costs_double(void) {
	checkrate("shields up doubles the bill",
		  warp_energy(1.5, 4.0, 1), warp_energy(1.5, 4.0, 0)*2.0);
}

/* "to travel at a speed of (warp factor squared)/10 quadrants per
 * stardate" -- sst.doc:1468; warp 10 is "100 times as fast" as warp 1
 * -- sst.doc:599. */
static void test_warp_speed_is_the_square_of_the_factor(void) {
	double one = warp_time(1.0, 1.0);

	checkrate("a quadrant at warp 1 takes ten stardates", one, 10.0);
	checkrate("warp 5 covers two and a half quadrants a stardate",
		  warp_time(1.0, 5.0), 0.4);
	checkrate("warp 10 is a hundred times as fast", warp_time(1.0, 10.0),
		  one/100.0);
	checkrate("warp 2 is four times as fast", warp_time(1.0, 2.0), one/4.0);
	checkrate("twice as far takes twice as long",
		  warp_time(2.0, 5.0), warp_time(1.0, 5.0)*2.0);
}

/* "The impulse engines require 20 units of energy to engage, plus 10
 * units per sector (100 units per quadrant) traveled. It does not cost
 * extra to move with the shields up." -- sst.doc:626-628 */
static void test_impulse_energy(void) {
	checkrate("engaging alone costs twenty", impulse_energy(0.0), 20.0);
	checkrate("a quadrant costs a hundred more",
		  impulse_energy(1.0), 20.0 + 100.0);
	checkrate("a sector costs ten more", impulse_energy(0.1), 20.0 + 10.0);
	checkrate("three quadrants", impulse_energy(3.0), 20.0 + 300.0);
	/* Nothing here takes a shields argument, which is the manual's
	   "it does not cost extra to move with the shields up" said in
	   the only way C can say it. */
}

/* "They move you at a speed of 0.95 sectors per stardate"
 * -- sst.doc:620 */
static void test_impulse_speed(void) {
	/* 0.95 sectors is 0.095 quadrants, so a stardate buys that far. */
	checkrate("a stardate carries you 0.95 sectors",
		  impulse_time(0.095), 1.0);
	checkrate("a whole quadrant takes rather longer",
		  impulse_time(1.0), 1.0/0.095);

	/* "which is the equivalent of a warp factor of about 0.975"
	   -- sst.doc:621. About: the two agree to within a twentieth of a
	   stardate over a whole quadrant, which is what "about" is doing. */
	checkclose("impulse is about warp 0.975", impulse_time(1.0),
		   warp_time(1.0, 0.975), 0.05);
}

/* "It costs 50 units of energy to raise shields, nothing to lower
 * them." -- sst.doc:646. "it costs you 200 units of energy to activate
 * this control" -- sst.doc:660.
 *
 * The manual prints both numbers twice -- again at sst.doc:1464-1466 --
 * so holding the constants to them is two sources agreeing rather than
 * a mirror. Their other worth is that the citation now sits at the
 * call site instead of a bare 50.0 in the middle of battle.c. */
static void test_shield_costs(void) {
	checkrate("raising shields costs fifty", SHIELD_RAISE_COST, 50.0);
	checkrate("the high-speed control costs two hundred",
		  FAST_SHIELD_COST, 200.0);
}

/* "If warp engines are damaged less than 10 stardates (undocked) you
 * can still go warp 4." -- sst.doc:572-573 */
static void test_warp_with_damaged_engines(void) {
	/* Sound engines will do anything in range. */
	checkint("warp 9 on sound engines", warp_verdict(9.0, 0.0), WARP_OK);

	/* "less than 10 stardates ... you can still go warp 4": any
	   damage at all caps the ship at four, and four is allowed. */
	checkint("warp 4 with the engines hurt",
		 warp_verdict(4.0, 5.0), WARP_OK);
	checkint("but not warp 5", warp_verdict(5.0, 5.0), WARP_LIMITED);
	checkint("a scratch is enough to cap it",
		 warp_verdict(5.0, 0.1), WARP_LIMITED);

	/* Ten stardates is where "less than 10" stops applying -- and the
	   manual stops there too, saying nothing about a ship damaged
	   exactly ten. The game has always allowed that one, so the test
	   holds it: this line is what the code decided, not what sst.doc
	   said, and without it "> 10" and ">= 10" are the same file. */
	checkint("nine stardates of damage still gives warp 4",
		 warp_verdict(4.0, 9.0), WARP_OK);
	checkint("ten exactly still gives warp 4",
		 warp_verdict(4.0, 10.0), WARP_OK);
	checkint("eleven gives nothing at all",
		 warp_verdict(4.0, 11.0), WARP_INOPERATIVE);
	checkint("and nothing at all means not even warp 1",
		 warp_verdict(1.0, 11.0), WARP_INOPERATIVE);
}

/* "Your minimum warp factor is 1.0 and your maximum warp factor is
 * 10.0" -- sst.doc:598 */
static void test_warp_factor_range(void) {
	checkint("warp 1 is the bottom of the range",
		 warp_verdict(1.0, 0.0), WARP_OK);
	checkint("below it is refused", warp_verdict(0.9, 0.0), WARP_TOO_SLOW);
	checkint("warp 10 is the top of the range",
		 warp_verdict(10.0, 0.0), WARP_OK);
	checkint("above it is refused", warp_verdict(10.1, 0.0), WARP_TOO_FAST);

	/* Damage is answered before the range is: engines that cannot run
	   are not a question about how fast the player asked to go. */
	checkint("inoperative beats out of range",
		 warp_verdict(99.0, 11.0), WARP_INOPERATIVE);
	checkint("and the cap beats it too",
		 warp_verdict(99.0, 1.0), WARP_LIMITED);
}

int main(void) {
	test_score_gains();
	test_score_surrender();
	test_score_losses();
	test_score_ships_lost();
	test_score_kill_rate();
	test_score_win_bonus();
	test_score_adds_up();
	test_promotion_threshold();
	test_promotion_is_earned_by_the_rate();
	test_promotion_penalties();
	test_promotion_counts_lost_ships();
	test_promotion_short_game();
	test_warp_costs_the_cube_of_the_factor();
	test_warp_with_shields_up_costs_double();
	test_warp_speed_is_the_square_of_the_factor();
	test_impulse_energy();
	test_impulse_speed();
	test_shield_costs();
	test_warp_with_damaged_engines();
	test_warp_factor_range();
	if (failures) {
		printf("%d test(s) FAILED\n", failures);
		return 1;
	}
	printf("All tests passed\n");
	return 0;
}
