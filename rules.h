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

#endif	/* SST_RULES_H */
