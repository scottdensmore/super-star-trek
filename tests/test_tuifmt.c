/* Unit tests for the TUI panel formatters (tuifmt.c).
 * Defines INCLUDED so sst.h instantiates the game-state globals here.
 */
#define INCLUDED
#include "../sst.h"
#include "../tui.h"

static int failures = 0;

static void check(const char *what, const char *got, const char *want) {
	if (strcmp(got, want) != 0) {
		failures++;
		printf("FAIL %s\n  want: \"%s\"\n  got:  \"%s\"\n", what, want, got);
	}
}

static const char *charnote(int v) {
	static char note[2][8];
	static int which;
	which = !which;
	if (v > ' ' && v < 127)
		snprintf(note[which], sizeof(note[which]), " ('%c')", v);
	else
		note[which][0] = '\0';
	return note[which];
}

static void checkint(const char *what, int got, int want) {
	if (got != want) {
		failures++;
		printf("FAIL %s\n  want: %d%s\n  got:  %d%s\n",
		       what, want, charnote(want), got, charnote(got));
	}
}

static void clearquad(char fill) {
	int i, j;
	for (i = 0; i <= 10; i++)
		for (j = 0; j <= 10; j++)
			quad[i][j] = fill;
}

static void basestate(void) {
	clearquad(IHDOT);
	memset(damage, 0, sizeof(damage));
	memset(d.galaxy, 0, sizeof(d.galaxy));
	memset(d.newstuf, 0, sizeof(d.newstuf));
	coordfixed = 0;
	condit = IHGREEN;
	quadx = 5; quady = 3;
	sectx = 5; secty = 5;
	quad[sectx][secty] = IHE;
	ship = IHE;
	d.date = 3421.4;
	d.remkl = 14;
	d.remtime = 12.35;
	energy = 2871.0;
	inshld = 2500.0;
	shield = 1250.0;
	shldup = FALSE;
	torps = 8;
	warpfac = 5.0;
	lsupres = 4.0;
}

static void test_quad_header(void) {
	char buf[FMTBUFLEN];
	basestate();
	fmt_quad_line(0, buf);
	check("quad header", buf, "    1 2 3 4 5 6 7 8 9 10");
}

static void test_quad_row(void) {
	char buf[FMTBUFLEN];
	basestate();
	quad[1][4] = IHSTAR;
	quad[1][7] = IHK;
	fmt_quad_line(1, buf);
	check("quad row 1", buf, " 1  . . . * . . K . . .");
	fmt_quad_line(5, buf);
	check("quad row with ship", buf, " 5  . . . . E . . . . .");
}

static void test_quad_masks_when_sensors_damaged(void) {
	char buf[FMTBUFLEN];
	basestate();
	quad[1][1] = IHK;	/* out of visual range: must be hidden */
	quad[4][5] = IHSTAR;	/* adjacent: must stay visible */
	damage[DSRSENS] = 1.0;
	fmt_quad_line(1, buf);
	check("masked far row", buf, " 1  - - - - - - - - - -");
	fmt_quad_line(4, buf);
	check("masked near row", buf, " 4  - - - . * . - - - -");

	/* Docked ships use the starbase's sensors: nothing is masked. */
	condit = IHDOCKED;
	fmt_quad_line(1, buf);
	check("docked row unmasked", buf, " 1  K . . . . . . . . .");
}

static void test_quad_coordfixed_masks_around_ship(void) {
	char buf[FMTBUFLEN];
	basestate();
	coordfixed = 1;
	quad[sectx][secty] = IHDOT;	/* with fixed coords, x is the column */
	sectx = 2; secty = 9;
	quad[sectx][secty] = IHE;
	damage[DSRSENS] = 1.0;
	fmt_quad_line(2, buf);
	check("coordfixed masked ship row", buf, " 9  . E . - - - - - - -");
	fmt_quad_line(10, buf);
	check("coordfixed masked far row", buf, " 1  - - - - - - - - - -");
}

static void test_quad_coordfixed_flips_rows(void) {
	char buf[FMTBUFLEN];
	basestate();
	coordfixed = 1;
	quad[2][10] = IHB;	/* with fixed coords, x is the column index */
	fmt_quad_line(1, buf);
	check("coordfixed top row", buf, "10  . B . . . . . . . .");
	fmt_quad_line(9, buf);
	check("coordfixed row 9", buf, " 2  . . . . . . . . . .");
}

/* tui.c starts colouring at STATLABEL, so every label must pad to
 * exactly that width -- otherwise colour would begin mid-value. */
static void test_status_labels_are_aligned(void) {
	char buf[FMTBUFLEN];
	int i;
	basestate();
	for (i = 1; i <= 10; i++) {
		fmt_status_line(i, buf);
		if ((int)strlen(buf) <= STATLABEL) continue;
		checkint("label does not end where colour starts",
			 buf[STATLABEL-1] == ' ' && buf[STATLABEL] != ' ', 1);
	}
}

static void test_status_lines(void) {
	char buf[FMTBUFLEN];
	basestate();
	fmt_status_line(1, buf);
	check("stardate", buf, "Stardate      3421.4");
	fmt_status_line(2, buf);
	check("condition", buf, "Condition     GREEN");
	fmt_status_line(3, buf);
	check("position", buf, "Position      5 - 3, 5 - 5");
	fmt_status_line(4, buf);
	check("life support", buf, "Life Support  ACTIVE");
	fmt_status_line(5, buf);
	check("warp factor", buf, "Warp Factor   5.0");
	fmt_status_line(6, buf);
	check("energy", buf, "Energy        2871.00");
	fmt_status_line(7, buf);
	check("torpedoes", buf, "Torpedoes     8");
	fmt_status_line(8, buf);
	check("shields", buf, "Shields       DOWN, 50% 1250.0 units");
	fmt_status_line(9, buf);
	check("klingons left", buf, "Klingons Left 14");
	fmt_status_line(10, buf);
	check("time left", buf, "Time Left     12.35");
}

/* condition_now() answers without writing to the game state, so a
 * panel repaint mid-turn cannot undo a condition the game set itself. */
static void test_condition_now(void) {
	char buf[FMTBUFLEN];
	basestate();
	checkint("quiet quadrant is green", condition_now(), IHGREEN);

	energy = 999.0;
	checkint("low energy is yellow", condition_now(), IHYELLOW);

	d.galaxy[quadx][quady] = 100;
	checkint("enemies beat low energy", condition_now(), IHRED);

	energy = 2871.0;
	checkint("enemies alone are red", condition_now(), IHRED);
	d.galaxy[quadx][quady] = 0;

	d.newstuf[quadx][quady] = 10;
	checkint("newly-arrived enemies are red", condition_now(), IHRED);
	d.newstuf[quadx][quady] = 0;

	/* Reading it must not have written it. */
	condit = IHDOCKED;
	(void)condition_now();
	checkint("condit is left alone", condit, IHDOCKED);

	/* Docked is where the ship is, not something to recompute --
	   srscan shows DOCKED here, so the panel has to as well. */
	fmt_status_line(2, buf);
	check("condition docked", buf, "Condition     DOCKED");
}

static void test_cell_class(void) {
	checkint("klingon is hostile", cell_class(IHK), CELL_HOSTILE);
	checkint("commander is hostile", cell_class(IHC), CELL_HOSTILE);
	checkint("super-commander is hostile", cell_class(IHS), CELL_HOSTILE);
	checkint("romulan is hostile", cell_class(IHR), CELL_HOSTILE);
	checkint("tholian is hostile", cell_class(IHT), CELL_HOSTILE);
	checkint("enterprise is ours", cell_class(IHE), CELL_SHIP);
	checkint("faerie queene is ours", cell_class(IHF), CELL_SHIP);
	checkint("starbase", cell_class(IHB), CELL_BASE);
	checkint("star", cell_class(IHSTAR), CELL_STAR);
	checkint("planet", cell_class(IHP), CELL_PLANET);
	checkint("tholian web", cell_class(IHWEB), CELL_WEB);
	checkint("the unknown thing", cell_class(IHQUEST), CELL_THING);
	/* A black hole is invisible on a scan by design; colouring it
	   would tell the player something the game withholds. */
	checkint("black hole stays invisible", cell_class(IHBLANK), CELL_PLAIN);
	checkint("masked cell", cell_class('-'), CELL_HIDDEN);
	checkint("empty space is plain", cell_class(IHDOT), CELL_PLAIN);
	checkint("a row label is plain", cell_class('7'), CELL_PLAIN);
}

/* The property colour must not break: with the sensors out, a row the
 * scan is hiding must not contain anything colour could tell apart.
 * Run over the formatter's own output, so it holds for whatever the
 * masking actually produces rather than for what we think it does.
 *
 * It only asks that nothing is colourable, not that hidden cells look
 * alike -- a mask emitting a mix of '.' and '-' would pass here while
 * still being readable. The exact-string tests above are what pin
 * that, so don't drop them believing this one covers it. */
static void test_masked_rows_are_colourless(void) {
	char buf[FMTBUFLEN];
	int i, j, leaked = 0;

	basestate();
	quad[1][1] = IHK;	/* all far from the ship at sector 5 - 5, */
	quad[1][2] = IHB;	/* so every one of them is hidden */
	quad[1][3] = IHSTAR;
	quad[1][4] = IHP;
	quad[10][9] = IHC;
	damage[DSRSENS] = 1.0;

	for (i = 1; i <= 10; i++) {
		if (i >= sectx-1 && i <= sectx+1)
			continue;	/* the 3x3 box around the ship is visible */
		fmt_quad_line(i, buf);
		for (j = 0; buf[j] != '\0'; j++) {
			int cls = cell_class(buf[j]);
			if (cls != CELL_PLAIN && cls != CELL_HIDDEN)
				leaked++;
		}
	}
	checkint("a hidden row still has something to colour", leaked, 0);
}

static void test_status_class(void) {
	basestate();
	checkint("quiet quadrant reads good", status_class(2), STAT_GOOD);
	energy = 999.0;
	checkint("low energy warns", status_class(2), STAT_WARN);
	d.galaxy[quadx][quady] = 100;
	checkint("enemies are bad news", status_class(2), STAT_BAD);
	condit = IHDOCKED;
	checkint("docked outranks the rest", status_class(2), STAT_DOCKED);

	basestate();
	checkint("life support is good", status_class(4), STAT_GOOD);
	damage[DLIFSUP] = 1.0;
	checkint("failing life support is bad", status_class(4), STAT_BAD);
	condit = IHDOCKED;
	checkint("the starbase makes it survivable", status_class(4), STAT_WARN);

	basestate();
	checkint("shields down warns", status_class(8), STAT_WARN);
	shldup = TRUE;
	checkint("shields up is good", status_class(8), STAT_GOOD);
	damage[DSHIELD] = 1.0;
	checkint("broken shields are bad", status_class(8), STAT_BAD);

	basestate();
	checkint("the stardate is just a number", status_class(1), STAT_PLAIN);
	checkint("so is the torpedo count", status_class(7), STAT_PLAIN);

	/* Nothing else in the game warns that the clock is running out. */
	intime = 7.0;
	d.remtime = 5.0;
	checkint("plenty of time left", status_class(10), STAT_PLAIN);
	d.remtime = 1.2;
	checkint("time getting short warns", status_class(10), STAT_WARN);
	d.remtime = 0.5;
	checkint("nearly out of time is bad", status_class(10), STAT_BAD);
	/* Negative remtime is what makes the intime guard load-bearing:
	   without it the ratio goes wrong way round and reads as BAD. */
	intime = 0.0;
	d.remtime = -1.0;
	checkint("no allotment, nothing to say", status_class(10), STAT_PLAIN);
}

static void test_status_variants(void) {
	char buf[FMTBUFLEN];
	basestate();
	/* The panel works the condition out from the game the way srscan
	   does, so drive it with enemies rather than assigning condit. */
	d.galaxy[quadx][quady] = 100;
	fmt_status_line(2, buf);
	check("condition red", buf, "Condition     RED");
	d.galaxy[quadx][quady] = 0;

	shldup = TRUE;
	fmt_status_line(8, buf);
	check("shields up", buf, "Shields       UP, 50% 1250.0 units");
	damage[DSHIELD] = 1.0;
	fmt_status_line(8, buf);
	check("shields damaged", buf, "Shields       DAMAGED, 50% 1250.0 units");

	damage[DLIFSUP] = 1.0;
	fmt_status_line(4, buf);
	check("life support damaged", buf, "Life Support  DAMAGED, reserves=4.00");
	condit = IHDOCKED;
	fmt_status_line(4, buf);
	check("life support docked", buf, "Life Support  DAMAGED, by starbase");
}

int main(void) {
	test_quad_header();
	test_quad_row();
	test_quad_masks_when_sensors_damaged();
	test_quad_coordfixed_masks_around_ship();
	test_quad_coordfixed_flips_rows();
	test_condition_now();
	test_cell_class();
	test_masked_rows_are_colourless();
	test_status_class();
	test_status_labels_are_aligned();
	test_status_lines();
	test_status_variants();
	if (failures) {
		printf("%d test(s) FAILED\n", failures);
		return 1;
	}
	printf("All tests passed\n");
	return 0;
}
