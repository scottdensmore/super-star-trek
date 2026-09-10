#include "cmdtab.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

static command_def_t commands[] = {
    /* 0 */
    {"srscan", "SRSCAN [no]", "Short-range sensor scan of current quadrant",
     "SRSCAN (or SRSCAN NO for scan without status report)",
     "  Mnemonic:  SRSCAN", CMD_CAT_SENSORS, 1, 1, 0},
    /* 1 */
    {"lrscan", "LRSCAN", "Long-range sensor scan of neighboring quadrants",
     "LRSCAN",
     "  Mnemonic:  LRSCAN", CMD_CAT_SENSORS, 1, 1, 1},
    /* 2 */
    {"phasers", "PHASERS [manual|automatic] [amount] [distribution]", "Fire ship phaser banks",
     "PHASERS automatic 500 (or PHASERS manual 300 200)",
     "  Mnemonic:  PHASERS", CMD_CAT_COMBAT, 1, 1, 2},
    /* 3 */
    {"photons", "PHOTONS [c] [c] [c]", "Fire photon torpedoes along course bearings",
     "PHOTONS 3 (Fire one torpedo along course 3, North)",
     "  Mnemonic:  PHOTON", CMD_CAT_COMBAT, 1, 1, 3},
    /* 4 */
    {"move", "MOVE [manual|automatic] <course> <distance>", "Navigate Enterprise using warp engines",
     "MOVE manual 1 2.5 (Course 1=East, distance=2.5 quadrants)",
     "  Mnemonic:  MOVE", CMD_CAT_NAV, 1, 1, 4},
    /* 5 */
    {"shields", "SHIELDS [up|down|transfer] [amount]", "Raise, lower, or transfer energy to shields",
     "SHIELDS up (or SHIELDS +300 to transfer 300 units)",
     "  Mnemonic:  SHIELDS", CMD_CAT_COMBAT, 1, 1, 5},
    /* 6 */
    {"dock", "DOCK", "Dock with adjacent Starbase for resupply and repairs",
     "DOCK (Must be adjacent to a starbase in quadrant)",
     "  Mnemonic:  DOCK", CMD_CAT_SHIP, 1, 1, 6},
    /* 7 */
    {"damages", "DAMAGES", "Display damage report for all ship devices",
     "DAMAGES",
     "  Mnemonic:  DAMAGES", CMD_CAT_SHIP, 1, 1, 7},
    /* 8 */
    {"chart", "CHART", "Display star chart of known quadrants in galaxy",
     "CHART",
     "  Mnemonic:  CHART", CMD_CAT_SENSORS, 1, 1, 8},
    /* 9 */
    {"impulse", "IMPULSE [manual] <course> <distance>", "Sub-light sub-warp impulse engine movement",
     "IMPULSE 3 2 (Move 2 sectors along course 3 North)",
     "  Mnemonic:  IMPULSE", CMD_CAT_NAV, 1, 1, 9},
    /* 10 */
    {"rest", "REST <stardates>", "Wait in place to repair systems without moving",
     "REST 1.5 (Rest for 1.5 stardates)",
     "  Mnemonic:  REST", CMD_CAT_NAV, 1, 1, 10},
    /* 11 */
    {"warp", "WARP <factor>", "Set warp speed factor (1.0 - 10.0)",
     "WARP 6.0 (Set cruising speed to Warp 6)",
     "  Mnemonic:  WARP", CMD_CAT_NAV, 1, 1, 11},
    /* 12 */
    {"status", "STATUS", "Display ship status readings and resources",
     "STATUS",
     "  Mnemonic:  STATUS", CMD_CAT_SHIP, 1, 1, 12},
    /* 13 */
    {"sensors", "SENSORS", "Readings from short and long-range sensors",
     "SENSORS",
     "  Mnemonic:  SENSORS", CMD_CAT_SENSORS, 1, 1, 13},
    /* 14 */
    {"orbit", "ORBIT", "Enter standard orbit around planet in current quadrant",
     "ORBIT (Must be adjacent to a planet)",
     "  Mnemonic:  ORBIT", CMD_CAT_SHIP, 1, 1, 14},
    /* 15 */
    {"transport", "TRANSPORT", "Use transporter to beam landing party to/from planet",
     "TRANSPORT (Must be in standard orbit)",
     "  Mnemonic:  TRANSPORT", CMD_CAT_SHIP, 1, 1, 15},
    /* 16 */
    {"mine", "MINE", "Mine dilithium crystals from planet surface",
     "MINE (Landing party must be on planet surface)",
     "  Mnemonic:  MINE", CMD_CAT_SHIP, 1, 1, 16},
    /* 17 */
    {"crystals", "CRYSTALS", "Install mined dilithium crystals into warp engines",
     "CRYSTALS",
     "  Mnemonic:  CRYSTALS", CMD_CAT_SHIP, 1, 1, 17},
    /* 18 */
    {"shuttle", "SHUTTLE [up|down]", "Deploy or retrieve shuttlecraft Galileo",
     "SHUTTLE down (Fly shuttlecraft to planet)",
     "  Mnemonic:  SHUTTLE", CMD_CAT_SHIP, 1, 1, 18},
    /* 19 */
    {"planets", "PLANETS", "List status and survey results of known planets",
     "PLANETS",
     "  Mnemonic:  PLANETS", CMD_CAT_SYSTEM, 1, 1, 19},
    /* 20 */
    {"request", "REQUEST", "Request sensor status update and tactical report",
     "REQUEST",
     "  Mnemonic:  REQUEST", CMD_CAT_SHIP, 1, 1, 20},
    /* 21 */
    {"report", "REPORT", "Display mission status report and score breakdown",
     "REPORT",
     "  Mnemonic:  REPORT", CMD_CAT_SYSTEM, 1, 1, 21},
    /* 22 */
    {"computer", "COMPUTER", "Consult library computer for course and navigation ETA",
     "COMPUTER",
     "  Mnemonic:  COMPUTER", CMD_CAT_SYSTEM, 1, 1, 22},
    /* 23 */
    {"commands", "COMMANDS", "List all legal commands by category",
     "COMMANDS",
     " ABBREV", CMD_CAT_SYSTEM, 1, 1, 23},
    /* 24 */
    {"emexit", "EMEXIT", "Emergency exit and quick game save",
     "EMEXIT",
     "  Mnemonic:  EMEXIT", CMD_CAT_SYSTEM, 1, 1, 24},
    /* 25 */
    {"probe", "PROBE [armed] <course> <distance>", "Launch deep space telemetry/warhead probe",
     "PROBE armed 1 5 (Launch armed probe course 1 distance 5)",
     "  Mnemonic:  PROBE", CMD_CAT_SENSORS, 1, 1, 25},
    /* 26 */
    {"cloak", "CLOAK [on|off]", "Engage or disengage Romulan cloaking device",
     "CLOAK on (or CLOAK off)",
     "  Mnemonic:  CLOAK", CMD_CAT_COMBAT, 1, 1, 26},
    /* 27 */
    {"capture", "CAPTURE", "Attempt to demand surrender of disabled enemy ship",
     "CAPTURE",
     "  Mnemonic:  CAPTURE", CMD_CAT_SHIP, 1, 1, 27},
    /* 28 */
    {"score", "SCORE", "Show current game score and tournament rating",
     "SCORE",
     "  Mnemonic:  SCORE", CMD_CAT_SYSTEM, 1, 1, 28},
    /* 29 */
    {"abandon", "ABANDON", "Abandon ship and escape in shuttlecraft",
     "ABANDON",
     "  Mnemonic:  ABANDON", CMD_CAT_SYSTEM, 0, 1, 29},
    /* 30 */
    {"destruct", "DESTRUCT", "Initiate self-destruct sequence",
     "DESTRUCT (Requires password confirmation)",
     "  Mnemonic:  DESTRUCT", CMD_CAT_SYSTEM, 0, 1, 30},
    /* 31 */
    {"freeze", "FREEZE [filename]", "Save current game state to file",
     "FREEZE mygame (or FREEZE mygame.trk; 1-9 chars, starts A-Z; thaw with FROZEN)",
     "  Mnemonic:  FREEZE", CMD_CAT_SYSTEM, 0, 1, 31},
    /* 32 */
    {"deathray", "DEATHRAY", "Fire experimental secret weapon (last resort)",
     "DEATHRAY",
     "  Mnemonic:  DEATHRAY", CMD_CAT_SHIP, 0, 1, 32},
    /* 33 */
    {"debug", "DEBUG", "Access developer debug utilities",
     "DEBUG",
     "  Mnemonic:  DEBUG", CMD_CAT_SYSTEM, 0, 1, 33},
    /* 34 */
    {"call", "CALL", "Call Starfleet for emergency starbase tractor beam",
     "CALL",
     "  Mnemonic:  CALL", CMD_CAT_SHIP, 0, 1, 34},
    /* 35 */
    {"quit", "QUIT", "End the current mission and resign commission",
     "QUIT",
     "  Mnemonic:  QUIT", CMD_CAT_SYSTEM, 0, 1, 35},
    /* 36 */
    {"help", "HELP [command|topic]", "Display help, syntax, and manual documentation",
     "HELP MOVE (or HELP SCORING, or HELP TOPICS)",
     "  Mnemonic:  HELP", CMD_CAT_SYSTEM, 0, 1, 36}
};

#define NUM_CMDS ((int)(sizeof(commands)/sizeof(commands[0])))

static topic_def_t topics[] = {
    {"scoring", "Game Scoring System", "SCORING", "COMMAND ABBREVIATIONS"},
    {"tui", "Full-Screen TUI Mode & Resizing", "optional full-screen interface", "ACKNOWLEDGMENTS"},
    {"notes", "Miscellaneous Game Notes", "MISCELLANEOUS NOTES", "SCORING"},
    {"abbrev", "Command Abbreviations", "COMMAND ABBREVIATIONS", "MODIFICATIONS"},
    {"modifications", "Game Modifications", "MODIFICATIONS", "ACKNOWLEDGMENTS"}
};

#define NUM_TOPICS ((int)(sizeof(topics)/sizeof(topics[0])))

void cmdtab_init(void) {
#ifndef CLOAKING
    commands[26].enabled = 0;
#endif
#ifndef CAPTURE
    commands[27].enabled = 0;
#endif
#ifndef SCORE
    commands[28].enabled = 0;
#endif
#ifndef DEBUG
    commands[33].enabled = 0;
#endif
}

int cmdtab_count(void) {
    return NUM_CMDS;
}

const command_def_t *cmdtab_get(int index) {
    if (index < 0 || index >= NUM_CMDS) return NULL;
    return &commands[index];
}

static int strcase_cmp(const char *s1, const char *s2) {
    while (*s1 && *s2) {
        int c1 = tolower((unsigned char)*s1);
        int c2 = tolower((unsigned char)*s2);
        if (c1 != c2) return c1 - c2;
        s1++;
        s2++;
    }
    return tolower((unsigned char)*s1) - tolower((unsigned char)*s2);
}

static int strncase_cmp(const char *s1, const char *s2, size_t n) {
    while (n > 0 && *s1 && *s2) {
        int c1 = tolower((unsigned char)*s1);
        int c2 = tolower((unsigned char)*s2);
        if (c1 != c2) return c1 - c2;
        s1++;
        s2++;
        n--;
    }
    if (n == 0) return 0;
    return tolower((unsigned char)*s1) - tolower((unsigned char)*s2);
}

const command_def_t *cmdtab_lookup(const char *input) {
    if (input == NULL || *input == '\0') return NULL;
    size_t inlen = strlen(input);

    /* Check exact matches first */
    for (int i = 0; i < NUM_CMDS; i++) {
        if (!commands[i].enabled) continue;
        if (strcase_cmp(commands[i].name, input) == 0) {
            return &commands[i];
        }
    }

    /* Check prefix match for commands that allow abbreviation */
    for (int i = 0; i < NUM_CMDS; i++) {
        if (!commands[i].enabled || !commands[i].allow_abbrev) continue;
        if (strncase_cmp(commands[i].name, input, inlen) == 0) {
            return &commands[i];
        }
    }

    return NULL;
}

static int min3(int a, int b, int c) {
    int m = a;
    if (b < m) m = b;
    if (c < m) m = c;
    return m;
}

static int levenshtein_distance(const char *s1, const char *s2) {
    int len1 = (int)strlen(s1);
    int len2 = (int)strlen(s2);
    if (len1 > 32 || len2 > 32) return 99;

    int d[33][33];
    for (int i = 0; i <= len1; i++) d[i][0] = i;
    for (int j = 0; j <= len2; j++) d[0][j] = j;

    for (int i = 1; i <= len1; i++) {
        for (int j = 1; j <= len2; j++) {
            int cost = (tolower((unsigned char)s1[i - 1]) == tolower((unsigned char)s2[j - 1])) ? 0 : 1;
            d[i][j] = min3(d[i - 1][j] + 1,        /* deletion */
                           d[i][j - 1] + 1,        /* insertion */
                           d[i - 1][j - 1] + cost); /* substitution */
        }
    }
    return d[len1][len2];
}

const command_def_t *cmdtab_suggest(const char *input) {
    if (input == NULL || *input == '\0') return NULL;
    const command_def_t *best_match = NULL;
    int best_dist = 99;

    for (int i = 0; i < NUM_CMDS; i++) {
        if (!commands[i].enabled) continue;
        int dist = levenshtein_distance(input, commands[i].name);
        if (dist < best_dist) {
            best_dist = dist;
            best_match = &commands[i];
        }
    }

    if (best_dist <= 2) {
        return best_match;
    }
    return NULL;
}

int cmdtab_topic_count(void) {
    return NUM_TOPICS;
}

const topic_def_t *cmdtab_topic_get(int index) {
    if (index < 0 || index >= NUM_TOPICS) return NULL;
    return &topics[index];
}

const topic_def_t *cmdtab_topic_lookup(const char *input) {
    if (input == NULL || *input == '\0') return NULL;
    for (int i = 0; i < NUM_TOPICS; i++) {
        if (strcase_cmp(topics[i].name, input) == 0) {
            return &topics[i];
        }
    }
    return NULL;
}

static void (*printer_fn)(const char *) = NULL;

void cmdtab_set_printer(void (*fn)(const char *)) {
    printer_fn = fn;
}

static void print_line(const char *s) {
    if (printer_fn) {
        printer_fn(s);
    } else {
        printf("%s\n", s);
    }
}

void cmdtab_print_categories(void) {
    char buf[128];
    static const char *cat_names[] = {
        "Navigation", "Combat & Defense", "Sensors & Intelligence",
        "Ship Operations", "System & Game"
    };

    for (int c = 0; c < 5; c++) {
        snprintf(buf, sizeof(buf), "--- %s ---", cat_names[c]);
        print_line(buf);
        for (int i = 0; i < NUM_CMDS; i++) {
            if (!commands[i].enabled || commands[i].category != (cmd_category_t)c) continue;
            snprintf(buf, sizeof(buf), "  %-10s : %s", commands[i].name, commands[i].summary);
            print_line(buf);
        }
    }
}

void cmdtab_print_topics(void) {
    char buf[128];
    print_line("--- Manual Topics (Use HELP <topic>) ---");
    for (int i = 0; i < NUM_TOPICS; i++) {
        snprintf(buf, sizeof(buf), "  %-14s : %s", topics[i].name, topics[i].title);
        print_line(buf);
    }
}
