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

int main(void) {
	test_score_gains();
	test_score_surrender();
	test_score_losses();
	test_score_ships_lost();
	test_score_kill_rate();
	test_score_win_bonus();
	test_score_adds_up();
	if (failures) {
		printf("%d test(s) FAILED\n", failures);
		return 1;
	}
	printf("All tests passed\n");
	return 0;
}
