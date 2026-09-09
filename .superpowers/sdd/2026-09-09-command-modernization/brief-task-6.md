# Task Brief: Task 6 - Modernize Interactive Prompts with Guidance & Examples

## Scope & Target Files
- Modify: `moving.c`
- Modify: `battle.c`
- Modify: `setup.c`

## Requirements
Follow the implementation plan `docs/superpowers/plans/2026-09-09-command-modernization.md` (Task 6) and spec `docs/superpowers/specs/2026-09-09-command-modernization-design.md`:

### 1. In `moving.c`
- In `getcd()`:
  - When prompting for manual or automatic navigation:
    Change `proutn("Manual or automatic- ");` to:
    `proutn("Manual or automatic navigation (e.g. 'manual 1 3')- ");`
  - When prompting for course:
    Change `proutn("Course (1-9) - ");` to:
    `proutn("Course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 1]- ");`
  - When prompting for distance:
    Change `proutn("Distance - ");` to:
    `proutn("Distance in quadrants or sectors [e.g. 2.5]- ");`
  - When prompting for sector / quadrant coordinates:
    Change `proutn("Destination sector or quadrant&sector- ");` to:
    `proutn("Destination (quadrant & sector, e.g. 4 5 1 2)- ");`
- In `setwrp()`:
  - Change `proutn("Warp factor- ");` to:
    `proutn("Warp factor (1.0 to 10.0, e.g. 6.0)- ");`
- In `probe()`:
  - Change `proutn("Arm NOVAMAX warhead?");` or `proutn("Arm NOVAMAX warhead? ");` to:
    `proutn("Arm NOVAMAX warhead (detonates at target)? (Y/N): ");`

### 2. In `battle.c`
- In `phasers()`:
  - When prompting for units to fire:
    Change `proutn("Number of units to fire- ");` to:
    `proutf("Units to fire (Energy: %.0f available, e.g. 500, 0 to cancel)- ", energy);` (or matching integer/float formatting of energy)
- In `photon()`:
  - When prompting for torpedo course:
    Change `proutn("Torpedo course (1-9)- ");` to:
    `proutn("Torpedo course (1-9; 1=E, 3=N, 5=W, 7=S) [e.g. 3]- ");`
- In `sheild()`:
  - When prompting for units to transfer:
    Change `proutn("Units to transfer- ");` to:
    `proutn("Units to transfer (+ to raise, - to drop, e.g. +300)- ");`

### 3. Verification
- Verify code compiles cleanly with no compiler warnings:
  `cmake --build --preset debug`
- Run smoke journey test:
  `ctest --preset debug -R '^(journey|journey-tui)$'`
- Check line endings:
  `ctest --preset debug -R '^lineendings$'`
- Commit with message: `feat(ui): add examples and directional guidance to interactive prompts`
