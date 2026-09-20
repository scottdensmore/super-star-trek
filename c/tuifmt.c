#include "sst.h"
#include "tui.h"

/* Panel formatters for the full-screen interface. These mirror the
 * srscan() display exactly -- same grid layout, same status fields,
 * and the same masking when the short-range sensors are damaged --
 * so the panels never show the player more than a scan would.
 * Kept free of curses calls so they can be unit tested. */

/* The condition the ship is in, worked out without writing it down.
 *
 * newcnd() stores the same answer in the global, which is fine when
 * the player asks for a scan between turns. The panel repaints in the
 * middle of one, and moving.c sets RED by hand before a tractor beam
 * takes hold -- a refresh that recomputed the global would hand that
 * back to GREEN just as the game decided the player was in trouble.
 * It lives here so the formatters stay testable on their own. */
/* Whether the grid is hiding anything, which is the one condition
 * srscan() prints a heading for. The panel has no room for a heading,
 * so tui.c captions the box instead -- but the test belongs here, next
 * to the masking it describes, so the two cannot drift apart. Docking
 * lends the ship the starbase's sensors, hence the second half. */
int sensors_masked(void) {
	return damage[DSRSENS] != 0.0 && condit != IHDOCKED;
}

int condition_now(void) {
	if (d.galaxy[quadx][quady] > 99 || d.newstuf[quadx][quady] > 9)
		return IHRED;
	if (energy < 1000.0)
		return IHYELLOW;
	return IHGREEN;
}

/* Classify a cell the panel is about to draw, by the character itself.
 *
 * Taking the character rather than the quadrant contents is what keeps
 * colour honest: a cell the sensors are hiding has already become '-'
 * by the time it gets here, so there is nothing left to leak. */
int cell_class(char c) {
	switch (c) {
		case IHK: case IHC: case IHS: case IHR: case IHT:
			return CELL_HOSTILE;
		case IHE: case IHF:	return CELL_SHIP;
		case IHB:		return CELL_BASE;
		case IHSTAR:		return CELL_STAR;
		case IHP:		return CELL_PLANET;
		case IHWEB:		return CELL_WEB;
		case IHQUEST:		return CELL_THING;
		/* A black hole (IHBLANK) is deliberately absent: it shows as
		   empty space on a short-range scan, which is the whole trap.
		   Marking it would change what the game tells the player, not
		   how it looks. */
		case '-':		return CELL_HIDDEN;
		default:		return CELL_PLAIN;
	}
}

/* Whether a status line is worth reading twice: the fields that tell a
 * player something is wrong, and the two that say it is not. */
int status_class(int i) {
	switch (i) {
		case 2:		/* condition */
			if (condit == IHDOCKED) return STAT_DOCKED;
			switch (condition_now()) {
				case IHRED:	return STAT_BAD;
				case IHYELLOW:	return STAT_WARN;
				default:	return STAT_GOOD;
			}
		case 4:		/* life support */
			if (damage[DLIFSUP] == 0.0) return STAT_GOOD;
			return condit == IHDOCKED ? STAT_WARN : STAT_BAD;
		case 8:		/* shields */
			if (damage[DSHIELD] != 0) return STAT_BAD;
			return shldup ? STAT_GOOD : STAT_WARN;
		case 10:	/* time left */
			/* The one number nothing else warns about: running the
			   clock out loses the game, and no event or line of
			   prose ever mentions it approaching. */
			if (intime <= 0.0) return STAT_PLAIN;
			if (d.remtime < 0.10*intime) return STAT_BAD;
			if (d.remtime < 0.20*intime) return STAT_WARN;
			return STAT_PLAIN;
		default:
			return STAT_PLAIN;
	}
}

void fmt_quad_line(int i, char *buf) {
	char *cp = buf;
	int j, row;
	int goodscan = !sensors_masked();

	if (i == 0) {
		strcpy(buf, "    1 2 3 4 5 6 7 8 9 10");
		return;
	}
	row = coordfixed ? 11-i : i;
	cp += sprintf(cp, "%2d ", row);
	for (j = 1; j <= 10; j++) {
		char c;
		int visible;
		if (coordfixed) {	/* x is the column index */
			c = quad[j][row];
			visible = abs(row-secty) <= 1 && abs(j-sectx) <= 1;
		} else {
			c = quad[row][j];
			visible = abs(row-sectx) <= 1 && abs(j-secty) <= 1;
		}
		if (!goodscan && !visible)
			c = '-';
		if (c < ' ') c = ' ';	/* before the game state exists */
		cp += sprintf(cp, " %c", c);
	}
}

void fmt_status_line(int i, char *buf) {
	char *cp;
	switch (i) {
		case 1:
			sprintf(buf, "Stardate      %.1f", d.date);
			break;
		case 2:
			/* Docked is a fact about where the ship is, not a
			   condition to recompute. */
			switch (condit == IHDOCKED ? IHDOCKED : condition_now()) {
				case IHRED: cp = "RED"; break;
				case IHGREEN: cp = "GREEN"; break;
				case IHYELLOW: cp = "YELLOW"; break;
				case IHDOCKED: cp = "DOCKED"; break;
				default: cp = "----"; break;
			}
			sprintf(buf, "Condition     %s", cp);
#ifdef CLOAKING
			if (iscloaked) strcat(buf, ", CLOAKED");
#endif
			break;
		case 3:
			sprintf(buf, "Position      %d - %d, %d - %d",
					quadx, quady, sectx, secty);
			break;
		case 4:
			if (damage[DLIFSUP] != 0.0) {
				if (condit == IHDOCKED)
					sprintf(buf, "Life Support  DAMAGED, by starbase");
				else
					sprintf(buf, "Life Support  DAMAGED, reserves=%4.2f",
							lsupres);
			}
			else
				sprintf(buf, "Life Support  ACTIVE");
			break;
		case 5:
			sprintf(buf, "Warp Factor   %.1f", warpfac);
			break;
		case 6:
			sprintf(buf, "Energy        %.2f", energy);
			break;
		case 7:
			sprintf(buf, "Torpedoes     %d", torps);
			break;
		case 8:
			if (damage[DSHIELD] != 0)
				cp = "DAMAGED,";
			else if (shldup)
				cp = "UP,";
			else
				cp = "DOWN,";
			sprintf(buf, "Shields       %s %d%% %.1f units", cp,
					(int)((100.0*shield)/inshld + 0.5), shield);
			break;
		case 9:
			sprintf(buf, "Klingons Left %d", d.remkl);
			break;
		case 10:
			sprintf(buf, "Time Left     %.2f", d.remtime);
			break;
		default:
			*buf = 0;
			break;
	}
}
