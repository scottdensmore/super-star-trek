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

static void clearquad(char fill) {
	int i, j;
	for (i = 0; i <= 10; i++)
		for (j = 0; j <= 10; j++)
			quad[i][j] = fill;
}

static void basestate(void) {
	clearquad(IHDOT);
	memset(damage, 0, sizeof(damage));
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

static void test_status_variants(void) {
	char buf[FMTBUFLEN];
	basestate();
	condit = IHRED;
	fmt_status_line(2, buf);
	check("condition red", buf, "Condition     RED");

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
	test_status_lines();
	test_status_variants();
	if (failures) {
		printf("%d test(s) FAILED\n", failures);
		return 1;
	}
	printf("All tests passed\n");
	return 0;
}
