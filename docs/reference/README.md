# Reference notes

Facts about No Man's Sky save files and game mechanics that the code depends on, with the evidence behind each one. These notes are the record; plan documents under `docs/plans` cite them rather than restating them, and code comments point here.

| Document | Covers |
|----------|--------|
| [nms-save-notes.md](nms-save-notes.md) | the save file: container, key obfuscation, top-level layout, addresses, bases and base objects, fleet, inventory, wonders; what is decoded, what is not |
| [nms-game-notes.md](nms-game-notes.md) | the game: object and item IDs, growth times, capacities, power, fleet rules, wonders categories, coordinate systems, where to look things up |

## Evidence tags

Every fact line ends with a tag saying how it is known. Ranked from strongest to weakest:

| Tag | Meaning |
|-----|---------|
| `[verified: …]` | read from a real save **and** checked against something independent: a second save a known time later, the in-game screen, object positions, or a second field that carries the same value in another form. The tag says what the check was and when. |
| `[game-data: …]` | taken from the game's own data by internal ID (for example a recipe database entry for `^U_GENERATOR_S`), not yet confirmed in a save. |
| `[community: …]` | from a wiki, forum, or a third-party tool's documentation. Names the source. |
| `[inferred: …]` | a reading of the save that fits the sample but has not been tested independently. Says what would test it. |
| `[open]` | not understood; the line records what was seen. |

A save reading plus an independent observation outranks any document, including these. When a document and a save disagree, the save wins and the document gets corrected.

## Adding to these notes

Any session, human or agent, that establishes a decode rule, an object ID, a constant, or a mechanic records it here before the work is called done, with its tag. Corrections replace the wrong line; they do not sit beside it. A fact that moves up the ladder (an inference confirmed in-game) gets its tag rewritten to say so. Every list item and table row in the two notes must carry a tag; `make lint-docs` checks that and that every link resolves.

The method that produces a `verified` tag:

1. Dump the raw field with `nms raw <path>` (see the README) and note the value and the save time.
2. Write down what you think the value means and what it would do over time or in the game.
3. Test that against something independent: a second save a known interval later, the in-game screen, the object's position, or a sibling field.
4. Look the ID up in game data (sources listed in [nms-game-notes.md](nms-game-notes.md)).
5. Tag the rung honestly. A guess that fits one save is `inferred`, not `verified`.
6. Record it here.
