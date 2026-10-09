# Flint’s Dock artwork

Source reference: `dev/image(20261008-075307).png` (853×1280 RGBA).
The XML files in `dev/desing_codes` supply content and component relationships;
the raster supplies materials and artistic direction. No reference screenshot is
used as an application page or a panel with baked-in words.

## Inventory and decisions

Existing files were opened and inspected, including alpha channels and dimensions.
Names alone were not used to choose assets.

| Existing resource                      | Dimensions / transparency           | Decision                                                                                                                |
| -------------------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `illustrations/welcome-raptor.png`     | 2048×1731 RGB, opaque               | Green raptor, warm scene and baked FLINT’S DOCK text; unsuitable for the black/silver isolated reference portrait.      |
| `illustrations/success-raptor.png`     | 2304×1170 RGB, opaque               | Warm green raptor, wide composition; unsuitable portrait/crop.                                                          |
| `illustrations/orbital-station.png`    | 2048×1636 RGB, opaque               | Warm background and different station; cannot serve as the small isolated violet station.                               |
| `brand/pirate-emblem.png`              | 1254×1254 RGBA                      | Detailed cyan skull/swords, different from the specified emblem; retained in source, excluded from the active bundle.   |
| `frames/frame-s01.png`…`frame-s10.png` | About 270×541 RGBA                  | Low-resolution handset-shaped frames; unsuitable for flexible interior panel geometry.                                  |
| Old token / NFT thumbnails             | About 31px / 78×83px, mostly opaque | Insufficient resolution for category artwork. XML thumbnails remain in explicit preview rows only.                      |
| `design/*.svg` artwork                 | Vector, varying transparency        | Simplified XML art is no longer used for the hero, internal scene, category ornaments, station, swords or metal frames. |
| XML functional icons                   | Vector                              | Kept for wallet, checkbox, navigation, progress, emblem and success check.                                              |
| Existing multi-screen boards           | 1448×1086 / 2167×1760               | Earlier ten-screen design; useful historical context, not the current art reference.                                    |

## Replacement media

These are **generated reconstructions**, not the original extracted production
layers. They were generated with the built-in `image_gen` tool using the supplied
raster as the visual reference. Source outputs were kept intact and copied into
this directory. `generation.json` records generation sources and prompts (the
first three entries contain prompt summaries). PNG alpha was inspected and the
center of both border assets is transparent.

| Semantic key          | File                   | Size          | Composition / use                                                                                                              |
| --------------------- | ---------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `hero.raptor`         | `hero-raptor-ring.png` | 1254² RGBA    | Detailed black scaled raptor, textured hat, Solana badge and mechanical ring in one composition. No second ring/badge overlay. |
| `scene.station`       | `station-scene.png`    | 1024×1536 RGB | Planet, orbital city, ships and deck in a single text-free background.                                                         |
| `decoration.sabers`   | `crossed-sabers.png`   | 2172×724 RGBA | Shallow crossed silver sabers; independent, fixed-aspect panel anchor.                                                         |
| `category.scam`       | `category-scam.png`    | 1254² RGBA    | Purple bare skull.                                                                                                             |
| `category.nft`        | `category-nft.png`     | 1254² RGBA    | Gold embossed skull coin.                                                                                                      |
| `category.dust`       | `category-dust.png`    | 1254² RGBA    | Mint faceted crystal cluster. The generated facets are a reconstruction, not the exact original cubes.                         |
| `category.dead_token` | `category-dead.png`    | 1254² RGBA    | Metallic diagonal bone.                                                                                                        |
| `decoration.station`  | `orbital-station.png`  | 1254² RGBA    | Violet/cyan modular station with front thruster.                                                                               |
| `frame.violet`        | `frame-violet.png`     | 1254² RGBA    | Reusable metal border, transparent aperture.                                                                                   |
| `frame.cyan`          | `frame-cyan.png`       | 1254² RGBA    | Matching cyan metal border, transparent aperture.                                                                              |

The active registry is `src/shared/assets.ts`. Functional SVGs are allowlisted by
local glob patterns. Old art remains in source for reference but is not imported
by the runtime registry. Wallet metadata cannot supply media URLs.

Portraits and ornaments use `object-fit: contain`; the scene uses `cover` with an
explicit center crop. Frames use `border-image-slice: 26%` and fixed CSS border
widths. Straight rails stretch while corner pieces retain their proportions. Gold
and red accents derive from the shared violet border rather than separate copies.

## Exact source layers still missing

No original lossless standalone layers were supplied for:

- the reference raptor/hat, original porthole and badge;
- the original crossed swords;
- the separate planet, city, fleet and deck;
- the four original skull/coin/cube/bone ornaments;
- the original miniature orbital station;
- the original winged title pedestal and individual panel/nav/category skins.

Generated media covers these roles, but does not make them pixel-identical to the
reference. In particular, the title uses a shared nine-slice metal panel rather
than the original bespoke winged pedestal. Replace semantic registry entries with
licensed original layers when available; do not overlay rings or scene parts
already included in a composite image. The exact reference font is also unknown;
local Roboto Condensed is the XML-recommended Cyrillic substitute.
