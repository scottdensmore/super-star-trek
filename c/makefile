CFLAGS=     -O -DSCORE -DCAPTURE -DCLOAKING

.c.o:
	$(CC) $(CFLAGS) -c $<

OFILES=     sst.o finish.o reports.o setup.o osx.o moving.o battle.o events.o ai.o planets.o cmdtab.o rules.o tui.o tuifmt.o
HFILES=     sst.h cmdtab.h finish.h osx.h rules.h tui.h

sst:  $(OFILES)
	$(CC) -o sst $(OFILES) -lncurses -lm

clean:
	rm -f $(OFILES) sst

$(OFILES):  $(HFILES)
