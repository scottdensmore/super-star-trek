#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "cmdtab.h"

int main(void) {
    cmdtab_init();
    assert(cmdtab_count() >= 36);

    /* 1. Exact lookup */
    const command_def_t *cmd = cmdtab_lookup("move");
    assert(cmd != NULL);
    assert(strcmp(cmd->name, "move") == 0);
    assert(cmd->syntax != NULL && strlen(cmd->syntax) > 0);
    assert(cmd->summary != NULL && strlen(cmd->summary) > 0);
    assert(cmd->example != NULL && strlen(cmd->example) > 0);

    /* Case-insensitivity */
    cmd = cmdtab_lookup("MOVE");
    assert(cmd != NULL);
    assert(strcmp(cmd->name, "move") == 0);

    /* 2. Abbreviation lookup */
    const command_def_t *abbrev_cmd = cmdtab_lookup("m");
    assert(abbrev_cmd != NULL);
    assert(strcmp(abbrev_cmd->name, "move") == 0);

    abbrev_cmd = cmdtab_lookup("ph");
    assert(abbrev_cmd != NULL);
    assert(strcmp(abbrev_cmd->name, "phasers") == 0);

    /* Non-abbreviated command rejects prefix */
    assert(cmdtab_lookup("fre") == NULL);
    assert(cmdtab_lookup("freeze") != NULL);

    /* 3. Typos and suggestions (Task 2) */
    const command_def_t *sug = cmdtab_suggest("mvoe");
    assert(sug != NULL);
    assert(strcmp(sug->name, "move") == 0);

    sug = cmdtab_suggest("phaser");
    assert(sug != NULL);
    assert(strcmp(sug->name, "phasers") == 0);

    sug = cmdtab_suggest("shild");
    assert(sug != NULL);
    assert(strcmp(sug->name, "shields") == 0);

    assert(cmdtab_suggest("xyzzy999") == NULL);

    /* 4. Topics lookup (Task 3) */
    assert(cmdtab_topic_count() >= 4);

    const topic_def_t *top = cmdtab_topic_lookup("scoring");
    assert(top != NULL);
    assert(strcmp(top->name, "scoring") == 0);
    assert(strstr(top->doc_header, "SCORING") != NULL);

    top = cmdtab_topic_lookup("tui");
    assert(top != NULL);
    assert(strstr(top->title, "Full-Screen") != NULL);

    top = cmdtab_topic_lookup("unknown_topic");
    assert(top == NULL);

    /* 5. Completeness check */
    for (int i = 0; i < cmdtab_count(); i++) {
        const command_def_t *c = cmdtab_get(i);
        assert(c != NULL);
        if (c->enabled) {
            assert(c->name && strlen(c->name) > 0);
            assert(c->syntax && strlen(c->syntax) > 0);
            assert(c->summary && strlen(c->summary) > 0);
            assert(c->example && strlen(c->example) > 0);
        }
    }

    printf("PASS: all cmdtab unit tests passed!\n");
    return 0;
}
