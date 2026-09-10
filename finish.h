#ifndef SST_FINISH_H
#define SST_FINISH_H

#include <stddef.h>

/* Score sheet column alignment: every row right-aligns its score
 * at column 47 (#62, #76). */
#define SCORE_SHEET_WIDTH 47

/* 12 Dual-form metric format strings (plural and singular) */
#define SCORE_FMT_ROMKL_PLURAL      "%6d Romulan ships destroyed            %5d\n"
#define SCORE_FMT_ROMKL_SINGULAR    "%6d Romulan ship destroyed             %5d\n"

#define SCORE_FMT_ROMREM_PLURAL     "%6d Romulan ships surrendered          %5d\n"
#define SCORE_FMT_ROMREM_SINGULAR   "%6d Romulan ship surrendered           %5d\n"

#define SCORE_FMT_KILLK_PLURAL      "%6d ordinary Klingon ships destroyed   %5d\n"
#define SCORE_FMT_KILLK_SINGULAR    "%6d ordinary Klingon ship destroyed    %5d\n"

#define SCORE_FMT_KILLC_PLURAL      "%6d Klingon Commander ships destroyed  %5d\n"
#define SCORE_FMT_KILLC_SINGULAR    "%6d Klingon Commander ship destroyed   %5d\n"

#define SCORE_FMT_CAPTURE_PLURAL    "%6d Klingons captured                  %5d\n"
#define SCORE_FMT_CAPTURE_SINGULAR  "%6d Klingon captured                   %5d\n"

#define SCORE_FMT_STARKL_PLURAL     "%6d stars destroyed by your action     %5d\n"
#define SCORE_FMT_STARKL_SINGULAR   "%6d star destroyed by your action      %5d\n"

#define SCORE_FMT_PLANKL_PLURAL     "%6d planets destroyed by your action   %5d\n"
#define SCORE_FMT_PLANKL_SINGULAR   "%6d planet destroyed by your action    %5d\n"

#define SCORE_FMT_BASEKL_PLURAL     "%6d bases destroyed by your action     %5d\n"
#define SCORE_FMT_BASEKL_SINGULAR   "%6d base destroyed by your action      %5d\n"

#define SCORE_FMT_HELP_PLURAL       "%6d calls for help from starbase       %5d\n"
#define SCORE_FMT_HELP_SINGULAR     "%6d call for help from starbase        %5d\n"

#define SCORE_FMT_CASUAL_PLURAL     "%6d casualties incurred                %5d\n"
#define SCORE_FMT_CASUAL_SINGULAR   "%6d casualty incurred                  %5d\n"

#define SCORE_FMT_SHIP_PLURAL       "%6d ships lost or destroyed            %5d\n"
#define SCORE_FMT_SHIP_SINGULAR     "%6d ship lost or destroyed             %5d\n"

#define SCORE_FMT_VIOL_PLURAL       "%6d Treaty of Algeron violations       %5d\n"
#define SCORE_FMT_VIOL_SINGULAR     "%6d Treaty of Algeron violation        %5d\n"

/* Single-form metric format strings */
#define SCORE_FMT_SUPER_COMMANDER   "%6d Super-Commander ship destroyed     %5d\n"
#define SCORE_FMT_KILL_RATE         "%6.2f Klingon ships per stardate         %5d\n"
#define SCORE_FMT_TOTAL             "TOTAL SCORE                               %5d\n"

/* Killed penalty literal */
#define SCORE_LIT_KILLED_PENALTY    "Penalty for getting yourself killed        -200"

/* Winning bonus components */
#define SCORE_BONUS_PREFIX          "Bonus for winning "
#define SCORE_BONUS_NOVICE          "Novice game  "
#define SCORE_BONUS_FAIR            "Fair game    "
#define SCORE_BONUS_GOOD            "Good game    "
#define SCORE_BONUS_EXPERT          "Expert game  "
#define SCORE_BONUS_EMERITUS        "Emeritus game"
#define SCORE_BONUS_TAIL            "           %5d\n"

/* Chooses plural or singular format string based on count */
static inline const char *score_line_fmt(int count, const char *plural, const char *singular) {
	return (count > 1) ? plural : singular;
}

enum score_row_kind {
	SCORE_ROW_INT_PAIR,     /* %6d ... %5d */
	SCORE_ROW_DOUBLE_INT,   /* %6.2f ... %5d */
	SCORE_ROW_INT_SINGLE    /* TOTAL SCORE ... %5d */
};

struct score_format_entry {
	const char *fmt;
	enum score_row_kind kind;
	const char *label;
};

static inline int score_format_count(void) {
	return 27;
}

static inline struct score_format_entry score_format_at(int idx) {
	static const struct score_format_entry entries[27] = {
		{ SCORE_FMT_ROMKL_PLURAL,     SCORE_ROW_INT_PAIR,   "Romulan ships destroyed (plural)" },
		{ SCORE_FMT_ROMKL_SINGULAR,   SCORE_ROW_INT_PAIR,   "Romulan ship destroyed (singular)" },
		{ SCORE_FMT_ROMREM_PLURAL,    SCORE_ROW_INT_PAIR,   "Romulan ships surrendered (plural)" },
		{ SCORE_FMT_ROMREM_SINGULAR,  SCORE_ROW_INT_PAIR,   "Romulan ship surrendered (singular)" },
		{ SCORE_FMT_KILLK_PLURAL,     SCORE_ROW_INT_PAIR,   "ordinary Klingon ships destroyed (plural)" },
		{ SCORE_FMT_KILLK_SINGULAR,   SCORE_ROW_INT_PAIR,   "ordinary Klingon ship destroyed (singular)" },
		{ SCORE_FMT_KILLC_PLURAL,     SCORE_ROW_INT_PAIR,   "Klingon Commander ships destroyed (plural)" },
		{ SCORE_FMT_KILLC_SINGULAR,   SCORE_ROW_INT_PAIR,   "Klingon Commander ship destroyed (singular)" },
		{ SCORE_FMT_SUPER_COMMANDER,  SCORE_ROW_INT_PAIR,   "Super-Commander ship destroyed" },
		{ SCORE_FMT_KILL_RATE,        SCORE_ROW_DOUBLE_INT, "Klingon ships per stardate" },
		{ SCORE_FMT_CAPTURE_PLURAL,   SCORE_ROW_INT_PAIR,   "Klingons captured (plural)" },
		{ SCORE_FMT_CAPTURE_SINGULAR, SCORE_ROW_INT_PAIR,   "Klingon captured (singular)" },
		{ SCORE_FMT_STARKL_PLURAL,    SCORE_ROW_INT_PAIR,   "stars destroyed by your action (plural)" },
		{ SCORE_FMT_STARKL_SINGULAR,  SCORE_ROW_INT_PAIR,   "star destroyed by your action (singular)" },
		{ SCORE_FMT_PLANKL_PLURAL,    SCORE_ROW_INT_PAIR,   "planets destroyed by your action (plural)" },
		{ SCORE_FMT_PLANKL_SINGULAR,  SCORE_ROW_INT_PAIR,   "planet destroyed by your action (singular)" },
		{ SCORE_FMT_BASEKL_PLURAL,    SCORE_ROW_INT_PAIR,   "bases destroyed by your action (plural)" },
		{ SCORE_FMT_BASEKL_SINGULAR,  SCORE_ROW_INT_PAIR,   "base destroyed by your action (singular)" },
		{ SCORE_FMT_HELP_PLURAL,      SCORE_ROW_INT_PAIR,   "calls for help from starbase (plural)" },
		{ SCORE_FMT_HELP_SINGULAR,    SCORE_ROW_INT_PAIR,   "call for help from starbase (singular)" },
		{ SCORE_FMT_CASUAL_PLURAL,    SCORE_ROW_INT_PAIR,   "casualties incurred (plural)" },
		{ SCORE_FMT_CASUAL_SINGULAR,  SCORE_ROW_INT_PAIR,   "casualty incurred (singular)" },
		{ SCORE_FMT_SHIP_PLURAL,      SCORE_ROW_INT_PAIR,   "ships lost or destroyed (plural)" },
		{ SCORE_FMT_SHIP_SINGULAR,    SCORE_ROW_INT_PAIR,   "ship lost or destroyed (singular)" },
		{ SCORE_FMT_VIOL_PLURAL,      SCORE_ROW_INT_PAIR,   "Treaty of Algeron violations (plural)" },
		{ SCORE_FMT_VIOL_SINGULAR,    SCORE_ROW_INT_PAIR,   "Treaty of Algeron violation (singular)" },
		{ SCORE_FMT_TOTAL,            SCORE_ROW_INT_SINGLE, "TOTAL SCORE" }
	};
	if (idx >= 0 && idx < 27) return entries[idx];
	return (struct score_format_entry){ NULL, SCORE_ROW_INT_PAIR, NULL };
}

static inline int score_skill_count(void) {
	return 5;
}

static inline const char *score_skill_label(int skill_idx) {
	static const char * const labels[5] = {
		SCORE_BONUS_NOVICE,
		SCORE_BONUS_FAIR,
		SCORE_BONUS_GOOD,
		SCORE_BONUS_EXPERT,
		SCORE_BONUS_EMERITUS
	};
	if (skill_idx >= 0 && skill_idx < 5) return labels[skill_idx];
	return NULL;
}

#endif /* SST_FINISH_H */
