#include "sst.h"
#include "tui.h"

/* Panel formatters for the full-screen interface. These mirror the
 * srscan() display exactly -- same grid layout, same status fields,
 * and the same masking when the short-range sensors are damaged --
 * so the panels never show the player more than a scan would.
 * Kept free of curses calls so they can be unit tested. */

void fmt_quad_line(int i, char *buf) {
	char *cp = buf;
	int j, row;
	int goodscan = damage[DSRSENS] == 0.0 || condit == IHDOCKED;

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
			switch (condit) {
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
