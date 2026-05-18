# Changelog

## 0.1.3

- Attaching an issue to a board/sprint now works on every YouTrack instance.
  Previously it used a localized apply-command and failed on non-English
  instances (e.g. RU: "Неизвестная команда: Board").
- `board` and `sprint` accept either a name or an id, in any language and any
  casing (matched trimmed + Unicode case-insensitive). E.g. `црппо` resolves
  the `ЦРППО` board.
- A wrong/unknown `board` now lists the available boards; a wrong/ambiguous
  `sprint` lists the valid sprints for that board (a board exposes only its
  own sprints, so callers can't know them up front).
- Board/sprint resolution errors are reported as `invalid_params` instead of
  a generic internal error.
