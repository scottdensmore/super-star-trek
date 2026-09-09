#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cmdtab.h"

static int failures = 0;

static void check_true(const char *what, int condition) {
    if (!condition) {
        failures++;
        printf("FAIL %s\n", what);
    }
}

static void check_str(const char *what, const char *got, const char *want) {
    if (got == NULL && want == NULL) return;
    if (got == NULL || want == NULL || strcmp(got, want) != 0) {
        failures++;
        printf("FAIL %s\n  want: \"%s\"\n  got:  \"%s\"\n",
               what, want ? want : "(null)", got ? got : "(null)");
    }
}

static void check_not_null(const char *what, const void *ptr) {
    if (ptr == NULL) {
        failures++;
        printf("FAIL %s: expected non-NULL\n", what);
    }
}

static void check_null(const char *what, const void *ptr) {
    if (ptr != NULL) {
        failures++;
        printf("FAIL %s: expected NULL\n", what);
    }
}

int main(void) {
    cmdtab_init();
    check_true("cmdtab_count >= 36", cmdtab_count() >= 36);

    /* 1. Exact lookup */
    const command_def_t *cmd = cmdtab_lookup("move");
    check_not_null("lookup 'move'", cmd);
    if (cmd != NULL) {
        check_str("cmd->name is move", cmd->name, "move");
        check_true("cmd->syntax not empty", cmd->syntax != NULL && strlen(cmd->syntax) > 0);
        check_true("cmd->summary not empty", cmd->summary != NULL && strlen(cmd->summary) > 0);
        check_true("cmd->example not empty", cmd->example != NULL && strlen(cmd->example) > 0);
    }

    /* Case-insensitivity */
    cmd = cmdtab_lookup("MOVE");
    check_not_null("lookup 'MOVE'", cmd);
    if (cmd != NULL) {
        check_str("lookup 'MOVE' yields 'move'", cmd->name, "move");
    }

    /* 2. Abbreviation lookup */
    const command_def_t *abbrev_cmd = cmdtab_lookup("m");
    check_not_null("lookup 'm'", abbrev_cmd);
    if (abbrev_cmd != NULL) {
        check_str("lookup 'm' yields 'move'", abbrev_cmd->name, "move");
    }

    abbrev_cmd = cmdtab_lookup("ph");
    check_not_null("lookup 'ph'", abbrev_cmd);
    if (abbrev_cmd != NULL) {
        check_str("lookup 'ph' yields 'phasers'", abbrev_cmd->name, "phasers");
    }

    /* Non-abbreviated command rejects prefix */
    check_null("lookup 'fre'", cmdtab_lookup("fre"));
    check_not_null("lookup 'freeze'", cmdtab_lookup("freeze"));

    /* 3. Typos and suggestions */
    const command_def_t *sug = cmdtab_suggest("mvoe");
    check_not_null("suggest 'mvoe'", sug);
    if (sug != NULL) {
        check_str("suggest 'mvoe' yields 'move'", sug->name, "move");
    }

    sug = cmdtab_suggest("phaser");
    check_not_null("suggest 'phaser'", sug);
    if (sug != NULL) {
        check_str("suggest 'phaser' yields 'phasers'", sug->name, "phasers");
    }

    sug = cmdtab_suggest("shild");
    check_not_null("suggest 'shild'", sug);
    if (sug != NULL) {
        check_str("suggest 'shild' yields 'shields'", sug->name, "shields");
    }

    check_null("suggest 'xyzzy999'", cmdtab_suggest("xyzzy999"));

    /* 4. Topics lookup */
    check_true("cmdtab_topic_count >= 4", cmdtab_topic_count() >= 4);

    const topic_def_t *top = cmdtab_topic_lookup("scoring");
    check_not_null("topic lookup 'scoring'", top);
    if (top != NULL) {
        check_str("top->name is scoring", top->name, "scoring");
        check_true("top->doc_header has SCORING", strstr(top->doc_header, "SCORING") != NULL);
    }

    top = cmdtab_topic_lookup("tui");
    check_not_null("topic lookup 'tui'", top);
    if (top != NULL) {
        check_true("top->title has Full-Screen", strstr(top->title, "Full-Screen") != NULL);
    }

    check_null("topic lookup 'unknown_topic'", cmdtab_topic_lookup("unknown_topic"));

    /* 5. Completeness check */
    for (int i = 0; i < cmdtab_count(); i++) {
        const command_def_t *c = cmdtab_get(i);
        check_not_null("cmdtab_get entry", c);
        if (c != NULL && c->enabled) {
            check_true("c->name non-empty", c->name && strlen(c->name) > 0);
            check_true("c->syntax non-empty", c->syntax && strlen(c->syntax) > 0);
            check_true("c->summary non-empty", c->summary && strlen(c->summary) > 0);
            check_true("c->example non-empty", c->example && strlen(c->example) > 0);
        }
    }

    if (failures == 0) {
        printf("PASS: all cmdtab unit tests passed!\n");
        return 0;
    } else {
        printf("FAIL: %d check(s) failed in test_cmdtab\n", failures);
        return 1;
    }
}
