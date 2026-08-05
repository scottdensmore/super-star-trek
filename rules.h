/* Rules the manual states in numbers.
 *
 * Everything here computes and narrates nothing, so it can be asked a
 * question directly -- which is the point. tests/test_rules.c checks
 * these against sst.doc rather than against what the code happens to
 * return, so that when the two disagree somebody finds out.
 */
#ifndef SST_RULES_H
#define SST_RULES_H

/* The parts of the score sheet that are worked out rather than simply
 * counted. The per-line points -- ten a Klingon, fifty a commander --
 * are multiplications the sheet does as it prints them; what needs
 * computing is the kill rate, the bonus, how many ships were lost, who
 * surrendered, and the sum of the lot.
 *
 * Spec: sst.doc, "Scoring is fairly simple", lines 1352-1378.
 */
struct scoring {
	double perdate;		/* Klingon kills per stardate */
	int killrate;		/* 500 times that, rounded */
	int winbonus;		/* 100 a skill grade, if the game was won */
	int shipslost;		/* 0 the Enterprise, 1 the Faerie Queene, 2 both */
	int surrendered;	/* Romulans who gave up; only counts on a win */
	int total;		/* the number at the foot of the sheet */
};

/* inGame is true for the SCORE command, which reports a game still
 * being played; false when the game has ended. elapsed is the
 * stardates the game has run, before the minimum the manual applies. */
void score_compute(struct scoring *s, int inGame, double elapsed);

/* Whether Starfleet promotes the captain a grade, and what it asked
 * of them.
 *
 * Spec: sst.doc, "In addition to your score, you may also be promoted",
 * lines 1380-1387.
 */
struct promotion {
	double penalties;	/* points lost, as the threshold counts them */
	double needed;		/* the kill rate the next rank asks for */
	double achieved;	/* the kill rate earned; 0 for a game under
				   five stardates, which is not asked */
	int earned;		/* Starfleet promotes */
};

/* elapsed is the stardates the game ran. */
void promotion_compute(struct promotion *p, double elapsed);

/* What it costs to move, and how long it takes.
 *
 * Distances are in quadrants throughout, which is what the game's dist
 * holds; the manual talks in sectors where it is clearer to, and ten
 * sectors make a quadrant. The arguments are called quadrants and
 * factor rather than dist and warpfac because sst.h turns both of
 * those names into struct members -- a parameter called dist would
 * expand to a.dist and not compile.
 *
 * Spec: sst.doc:1467-1468 states both warp formulas outright, with
 * sst.doc:579 and 598-599 saying the same thing in words; impulse is
 * at sst.doc:620 and 626-628, restated at sst.doc:1469-1470.
 */
double warp_energy(double quadrants, double factor, double shieldsup);
double warp_time(double quadrants, double factor);
double impulse_energy(double quadrants);
double impulse_time(double quadrants);

/* "It costs 50 units of energy to raise shields, nothing to lower
 * them." -- sst.doc:646, restated at sst.doc:1464. */
#define SHIELD_RAISE_COST	(50.0)

/* "it costs you 200 units of energy to activate this control"
 * -- sst.doc:660, the high-speed shield control that lets phasers be
 * fired with the shields up. Restated at sst.doc:1465-1466. */
#define FAST_SHIELD_COST	(200.0)

#endif	/* SST_RULES_H */
