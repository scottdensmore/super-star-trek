#ifndef CMDTAB_H
#define CMDTAB_H

typedef enum {
    CMD_CAT_NAV,       /* Navigation: MOVE, IMPULSE, WARP, REST */
    CMD_CAT_COMBAT,    /* Weapons & Tactical: PHASERS, PHOTONS, SHIELDS, CLOAK */
    CMD_CAT_SENSORS,   /* Scans & Intelligence: SRSCAN, LRSCAN, CHART, SENSORS, PROBE */
    CMD_CAT_SHIP,      /* Ship Operations: STATUS, DAMAGES, DOCK, ORBIT, TRANSPORT, MINE, CRYSTALS, SHUTTLE, DEATHRAY, CALL, CAPTURE */
    CMD_CAT_SYSTEM     /* Game & System: COMPUTER, COMMANDS, REPORT, PLANETS, SCORE, FREEZE, ABANDON, DESTRUCT, QUIT, HELP, DEBUG, EMEXIT */
} cmd_category_t;

typedef struct {
    const char *name;          /* Canonical name, e.g. "move" */
    const char *syntax;        /* Usage syntax, e.g. "MOVE [manual|automatic] <course> <distance>" */
    const char *summary;       /* Short description, e.g. "Navigate Enterprise using warp engines" */
    const char *example;       /* Concrete example, e.g. "MOVE manual 1 3 (Course 1=East, distance 3)" */
    const char *doc_key;       /* Key in sst.doc, e.g. "  Mnemonic:  MOVE" */
    cmd_category_t category;   /* Subsystem classification */
    int allow_abbrev;          /* 1 if unique prefix matching is allowed */
    int enabled;               /* 1 if command is active in current build */
    int id;                    /* Canonical command ID */
} command_def_t;

typedef struct {
    const char *name;          /* Topic key, e.g. "scoring", "tui", "notes", "abbrev" */
    const char *title;         /* Display title, e.g. "Game Scoring System" */
    const char *doc_header;    /* Header match in sst.doc */
    const char *doc_end;       /* Terminator delimiter in sst.doc */
} topic_def_t;

void cmdtab_init(void);
int cmdtab_count(void);
const command_def_t *cmdtab_get(int index);
const command_def_t *cmdtab_lookup(const char *input);
const command_def_t *cmdtab_suggest(const char *input);

int cmdtab_topic_count(void);
const topic_def_t *cmdtab_topic_get(int index);
const topic_def_t *cmdtab_topic_lookup(const char *input);

void cmdtab_set_printer(void (*fn)(const char *));
void cmdtab_print_categories(void);
void cmdtab_print_topics(void);

#endif /* CMDTAB_H */
