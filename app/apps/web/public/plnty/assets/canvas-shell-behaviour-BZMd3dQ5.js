import{y as f,t as s,o as e,l as t,p as n,be as c,D as N,m as p,bY as x,aC as $,fa as ce}from"./main-DamfXklH.js";const ie="plnty-board-jiggle",he="plnty-board-jiggle-strong",le=300;function de(d,g,m){const E=K=>`${Number(K.toFixed(3))}deg`,_=K=>`0 ${Number(K.toFixed(3))}px`,j=g*.875;return`
@keyframes ${d} {
  0%   { rotate: ${E(-g)}; translate: ${_(-m)}; }
  25%  { rotate: ${E(j)}; translate: ${_(m)}; }
  50%  { rotate: ${E(g)}; translate: ${_(-m)}; }
  75%  { rotate: ${E(-j)}; translate: ${_(m)}; }
  100% { rotate: ${E(-g)}; translate: ${_(-m)}; }
}`}const Ae=de(ie,.24,.15),pe=de(he,.7,.5),Se=`${ie} ${le}ms linear infinite`,$e=`${he} ${le}ms linear infinite`,u="plnty-shell-row",Ne=["--sh-row-icon-gap","--sh-row-pad-y","--sh-row-pad-x","--sh-row-radius","--sh-fav-inset","--sh-fav-button","--sh-new-dot"],I="plnty-shell-row-inner",O="plnty-shell-row--favourited",i="plnty-shell-fav",V="plnty-shell-fav--pop",b="plnty-shell-pill",k="plnty-shell-pill--dark",X="plnty-shell-sel-spark",R="plnty-shell-new-dot",xe="plnty-shell-row-lock",Le="data-canvas-tools-panel",L="plnty-shell-row--dragging",M="plnty-shell-row--returning",F="plnty-shell-row--landing",H="plnty-kit-pill",a="plnty-kit-row",A="plnty-kit-row--hover",D="plnty-kit-row--dragging",h="plnty-kit-row--empty",o="plnty-kit-chain-row",r="plnty-kit-dominant-row",w="plnty-kit-chain-row-delete",y="data-chain-delete-armed",B="plnty-kit-rail-btn",z="plnty-kit-rail-logo",T="plnty-kit-bar-btn",P="plnty-kit-chip",q="plnty-kit-handle",J="plnty-kit-gen-thumb",ue="plnty-kit-gen-sweep",G="plnty-kit-slot-board",Z="plnty-kit-slot-cells-lit",S="data-drop-hover",Q="data-drop-tint",we=["[data-image-drop-zone]","[data-video-drop-zone]","[data-vector-drop-zone]","[data-audio-drop-zone]","[data-text-drop-zone]","[data-palette-drop-zone]"],l={cell:"--plnty-slot-cell",tile:"--plnty-slot-tile",tileInk:"--plnty-slot-tile-ink",wash:"--plnty-slot-wash",tint:"--plnty-slot-tint"},me="data-drop-leaving";function ee(...d){const g=`${e.hoverMs}ms ${e.hoverEase}`;return d.map(m=>`${m} var(${l.tint}, ${g})`).join(", ")}const C="plnty-kit-press",te="plnty-kit-press-face",v="plnty-kit-switch",W="plnty-kit-switch__knob",U="plnty-kit-switch__label",oe="plnty-kit-tip-in",Y="plnty-kit-hint-wave",ve=3200,ge=45,ae=2,fe=1,be=200,ke=150,ye=2,ne=180,Te=5,Ee=1.5,re=200,_e=280,Ie=3e3,Oe=`
/* ─── The rings a canvas element wears ────────────────────────────────────
   A selected picture's blue ring, its hover halo and its hairline are box-shadow
   spreads whose width is a screen size divided by the live zoom. Transitioning
   box-shadow itself (the halo's fade, 2026-09-20 morning) meant the zoom
   retargeted the tween every frame and the ring trailed the camera (Dor: "there
   is a delay when user zoom in and out - with the dynamic stroke thickness").
   So the SCREEN width and the colour are registered properties that transition,
   and the zoom is multiplied in outside them, untransitioned: the ring follows
   the camera the same frame and still grows in and fades out on its own clock.
   Registered here, once, for every element that draws one; the element writes
   the four on its own style and transitions them with RING_TRANSITION. */
@property --plnty-ring-w { syntax: '<length>'; inherits: false; initial-value: 0px; }
@property --plnty-ring-c { syntax: '<color>'; inherits: false; initial-value: transparent; }
@property --plnty-halo-w { syntax: '<length>'; inherits: false; initial-value: 0px; }
@property --plnty-halo-c { syntax: '<color>'; inherits: false; initial-value: transparent; }
/* ─── The Canvas kit ───────────────────────────────────────────────────── */
/* Pill / Chrome. The ring is the master's OUTSIDE 1px stroke; the second
   shadow is Canvas / Float, and on hover Canvas / Lift. data-still is the
   Label tone and a disabled pill: face and shadow stay, nothing rises. */
.${H} {
  background: var(${f.idle});
  color: var(${f.idleInk});
  box-shadow: 0 0 0 1px var(${f.idleLine}), var(--cp-shadow, ${s.float});
  transform: translateY(0);
  transition:
    transform ${e.hoverMs}ms ${e.hoverEase},
    background ${e.hoverMs}ms ${e.hoverEase},
    color ${e.hoverMs}ms ${e.hoverEase},
    box-shadow ${e.hoverMs}ms ${e.hoverEase};
}
.${H}:hover:not([data-still]),
.${H}[data-hover="true"]:not([data-still]) {
  background: var(${f.hover});
  color: var(${f.hoverInk});
  box-shadow: 0 0 0 1px var(${f.hoverLine}), var(--cp-shadow-hover, ${s.lift});
  transform: translateY(-${e.hoverLift}px);
}
/* Tool row. A control inside a list: it changes face, and rises 2 on a real
   hover (the operator's call, see the note above the constants). */
.${a} {
  box-sizing: border-box;
  background: ${t.row};
  border: 1px solid ${t.lineSoft};
  box-shadow: ${s.none};
  transform: translateY(0);
  transition:
    background ${e.hoverMs}ms ${e.hoverEase},
    border-color ${e.hoverMs}ms ${e.hoverEase},
    box-shadow ${e.hoverMs}ms ${e.hoverEase},
    transform ${e.hoverMs}ms ${e.hoverEase};
}
.${a}:hover,
.${a}.${A} {
  background: ${t.lifted};
  border-color: ${t.lineStrong};
  box-shadow: ${s.lift};
}
.${a}:hover:not(.${h}) {
  transform: translateY(-${e.rowLift}px);
}
/* State=Dragging: the row in the hand (master 703:36194). The +3 tilt is put
   on the clone by useToolRowGrab, not here, so the rotation can ease. */
.${a}.${D} {
  background: ${t.lifted};
  border-color: ${t.lineStrong};
  box-shadow: ${s.float};
}
/* State=Empty: the slot the row left (master 703:36203). colour/field, no
   stroke, Canvas / Pressed inside, and nothing drawn in it. */
.${a}.${h} {
  background: ${t.field};
  border-color: transparent;
  box-shadow: ${s.pressed};
}
.${a}.${h} > * { visibility: hidden; }
/* Chain row, the thumb row (chains round 9 D, 3339:26079), and the dominant tool
   row (canvas / plnty tools rail round 1, 3628:91), which was drawn with the
   same faces. Paper and the soft line at rest; the Tool row's hover and its
   rise, every line drawn as a ring so the row's words never move a pixel (the
   master counts its 1px in the layout). */
.${o},
.${r} {
  background: ${t.paper};
  box-shadow: inset 0 0 0 1px ${t.lineSoft}, ${s.none};
  transform: translateY(0);
  transition:
    background ${e.hoverMs}ms ${e.hoverEase},
    box-shadow ${e.hoverMs}ms ${e.hoverEase},
    transform ${e.hoverMs}ms ${e.hoverEase};
}
.${o}:hover,
.${o}.${A},
.${r}:hover,
.${r}.${A} {
  background: ${t.lifted};
  box-shadow: inset 0 0 0 1px ${t.lineStrong}, ${s.lift};
}
.${o}:hover:not(.${h}),
.${r}:hover:not(.${h}) {
  transform: translateY(-${e.rowLift}px);
}
.${o}.${D},
.${r}.${D} {
  background: ${t.lifted};
  box-shadow: inset 0 0 0 1px ${t.lineStrong}, ${s.float};
}
.${o}.${h},
.${r}.${h} {
  background: ${t.field};
  box-shadow: ${s.pressed};
}
.${o}.${h} > *,
.${r}.${h} > * { visibility: hidden; }
/* The chain row's X (operator, 2026-09-29: "a solution on how to delete
   chains", then "delete it with holding cmd only"): at the far end of the name,
   where a tool row's heart sits, and rising in as the heart does. It is up only
   while the row is hovered AND Cmd is held, Ctrl off a Mac (the column's
   CHAIN_DELETE_ARMED_ATTR), so a delete is never one stray click away. Out of
   the row otherwise (display, not opacity, as the kit's trailing marks leave
   it), so a long name keeps its width. Never on the row in the hand or the slot
   it left; the clone in the hand lives outside the column, so it never is. */
.${w} {
  display: none;
  width: ${n.iconSmall}px;
  height: ${n.iconSmall}px;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: auto;
  padding: 0;
  border: none;
  background: none;
  color: ${t.muted};
  transition: color ${e.hoverMs}ms ${e.hoverEase};
}
[${y}] .${o}:hover .${w},
[${y}] .${o}.${A} .${w} {
  display: flex;
  animation: plnty-shell-fav-in ${re}ms ${c.easeOut} forwards;
}
.${w}:hover { color: ${t.ink}; }
[${y}] .${o}.${D} .${w},
[${y}] .${o}.${h} .${w} { display: none; }
/* Rail button (master 703:36105 / 703:36108). */
.${B} {
  background: transparent;
  transition: background ${e.hoverMs}ms ${e.hoverEase};
}
.${B}:hover:not(:disabled) { background: ${t.pressedHover}; }
.${B}[aria-current="true"] { background: ${t.pressed}; }
/* The Logo lane's mark. It takes no plate on hover the way the buttons under
   it do: a plate would make it look like a tenth category. It grows instead
   (Dor, 2026-09-22: "have the plnty logo on the very left top grow a tiny
   bit when hovering"). A transform only, so the 35 lane never reflows. */
.${z} {
  transition: transform ${e.hoverMs}ms ${e.hoverEase};
}
.${z}:hover { transform: scale(${e.markGrow}); }
/* Bar button (master 703:36112 / 36115 / 36118). Active is the lilac pair,
   and the glyph inside reads lilac ink through currentColor. */
.${T} {
  background: transparent;
  color: ${t.ink};
  transition:
    background ${e.hoverMs}ms ${e.hoverEase},
    color ${e.hoverMs}ms ${e.hoverEase};
}
.${T}:hover:not([aria-pressed="true"]) { background: ${t.pressed}; }
.${T}[aria-pressed="true"] { background: ${t.lilac}; color: ${t.lilacInk}; }
/* The glyph inside rises glyphLift on hover while the button holds still
   (Dor, 2026-09-22: "have the icons jump 2px when hover, not rothko"). The
   active button's glyph stays put, the way the selected row does. The
   button's one child is the glyph (ShellGlyph's span). */
.${T} > * { transition: transform ${e.hoverMs}ms ${e.hoverEase}; }
.${T}:hover:not([aria-pressed="true"]) > * { transform: translateY(-${e.glyphLift}px); }
/* Action chip (master 703:36122 / 703:36126). */
.${P} {
  background: transparent;
  transition: background ${e.hoverMs}ms ${e.hoverEase};
}
.${P}:hover:not([aria-disabled="true"]),
.${P}[aria-pressed="true"],
.${P}[aria-expanded="true"] { background: ${t.pressed}; }
/* Every clickable thing in a tool card answers the pointer (Dor, 2026-09-22:
   "in general in the tools make sure every clickable element gets a hover
   state"). The uplift kit draws its faces inline, some fifteen of them on a
   dozen grounds, so the hover is a WASH laid over whatever face the button
   has: ink at 6% (the same step KIT_CHIP takes), or paper at 12% on an ink
   face (data-face="dark"), through background-image, which layers over
   the inline background instead of replacing it. The !important is what
   lets a stylesheet rule reach past an inline shorthand, which resets
   background-image to none. A button with no face of its own (the count's
   arrows, the dice) gets the wash as its face. Disabled stays put, and a
   segment that is already chosen never wears the class. */
.${C}:hover:not(:disabled):not([aria-disabled="true"]),
.${C}:hover:not(:disabled):not([aria-disabled="true"]) > .${te} {
  background-image: linear-gradient(rgba(26, 26, 25, 0.06), rgba(26, 26, 25, 0.06)) !important;
}
.${C}[data-face="dark"]:hover:not(:disabled):not([aria-disabled="true"]),
.${C}:hover:not(:disabled):not([aria-disabled="true"]) > .${te}[data-face="dark"] {
  background-image: linear-gradient(rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.12)) !important;
}
/* An empty slot under the pointer goes a step darker, so it reads as a
   thing to click (Dor, 2026-09-22: "when hover on this it becomes a bit
   darker to make sure the user understands it's clickable"). The slot's
   ground is an inline colour/slot on some fifty hosts; the checker board is
   the one child every empty slot draws, it covers the slot exactly and takes
   its corners (border-radius: inherit), so the wash goes on the board. Ink
   at 4%: the same step the card's outline takes under the pointer. A filled
   slot has no board, and no hover.

   ⚠ THE PARENT'S :hover IS NOT ENOUGH, AND ON A TOOL CARD IT NEVER FIRES
   (measured 2026-09-22, antagonist: pointer over the checker, board still
   transparent). A SmartArea draws its board inside the OVERLAY that sits on
   the slot, and an overlay is pointer-events: none so it cannot swallow the
   drop — an element that takes no pointer events is never :hover, so the
   parent rule had no element to match on. The element that DOES get the
   pointer is the drop zone itself, one or two hops up, and it names itself
   with the same attribute the drag bridges find it by (image-drag-bridge.ts
   and its four siblings), so that is what the hover hangs from here. A zone is
   never nested inside another zone — a multi-slot area stamps each slot its
   own zone and groups them with data-drop-group — so a descendant match
   lights exactly one board. */
.${G} {
  background: transparent;
  transition: ${ee("background")};
}
:hover > .${G},
${we.map(d=>`${d}:hover .${G}`).join(`,
`)} {
  background: rgba(26, 26, 25, 0.04);
}
/* A picture in the hand over the slot (660:42924). The board paints the
   wash as its own ground (the same property its pointer hover uses, so the
   zone gains no element: the first cut inserted a superellipse-cornered div
   under the board, and re-rasterising it behind a moving picture every
   frame is what dropped the drag's frame rate, Dor 2026-09-22), and its
   cells and tile go the kit's lilac with lilacInk words, the drawn colours
   rather than a tint. The vars are read by UpliftBoard with the resting
   colours as fallbacks. After the :hover rule on purpose: equal specificity,
   and the drop target wins the ground. */
[${S}] .${G} { background: var(${l.wash}, transparent); }
/* The drop's clock for all four parts (SLOT_VARS.tint): the fill fades in,
   and lingers on the way out. The pointer hover keeps its own. */
[${S}] { ${l.tint}: ${N.canvasTools.dragHover.transition.dropZoneFadeInMs}ms ${N.canvasTools.dragHover.transition.dropZoneFadeEasing}; }
[${me}] { ${l.tint}: ${N.canvasTools.dragHover.transition.dropZoneFadeOutMs}ms ${N.canvasTools.dragHover.transition.dropZoneFadeEasing}; }
[${S}="valid"] {
  ${l.tile}: ${t.lilac};
  ${l.tileInk}: ${t.lilacInk};
}
/* A picture of the wrong kind over the slot (a clip on an image slot, a
   picture on a video slot): the same drawing in the brand's red pair, its
   light red for the cells and the tile, its dark red for the word, on the
   wrong-input wash (Dor, 2026-09-24: "make the colors of the checkers also
   change to our red (the light one) keep the very light red background the
   same"). Until then the checker stayed grey under the red wash. */
[${S}="invalid"] {
  ${l.tile}: ${t.redFill};
  ${l.tileInk}: ${t.redInk};
}
/* The lit cells read the zone's last tint, not its live hover, so they fade
   out in the colour they faded in (SLOT_DROP_TINT_ATTR). */
[${Q}="valid"] { ${l.cell}: ${t.lilac}; }
[${Q}="invalid"] { ${l.cell}: ${t.redFill}; }
/* ⚠ THE CELLS CROSS-FADE AS ONE LAYER. THEY MUST NEVER TRANSITION THEIR OWN
   FILL (measured 2026-09-22, and it is the whole reason this class exists). A
   board is about 138 rects, so a transition on the fill of the board's rects
   is 138 simultaneous transitions, each re-running style and re-painting the
   SVG on every frame of the 160ms. On /dev/uplift at 6x CPU throttle, base
   frame 8.3ms: hover in and out stretched 15-16 frames and the worst was
   67-83ms. The same hover with the cells cross-faded instead stretched 0-4
   frames, worst 9.3-17ms. Dor felt it as the drag dropping frames the moment
   the picture reached a placeholder.

   So UpliftBoard draws the cells TWICE: the resting checker, and a lilac copy
   over it at opacity 0. Only that copy's opacity moves, which the compositor
   owns, so the hover costs no paint at all. It is built when the board mounts,
   never on hover: a layer created at the moment the picture arrives is the
   same hitch by another name (measured 291ms on the frame that inserted it). */
.${Z} {
  position: absolute;
  inset: 0;
  opacity: 0;
  transition: ${ee("opacity")};
}
/* Both states: the copy wears lilac for a picture that fits and red for one
   that does not (the tint rules above). */
[${S}] .${Z} { opacity: 1; }
/* Selection handle (master 703:36131): 6 square, colour/row, a 1.5
   colour/selection stroke outside, radius 1, centred in its hit box. */
.${q} { position: absolute; }
.${q}::after {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  width: ${n.handle}px;
  height: ${n.handle}px;
  margin-left: -${n.handle/2}px;
  margin-top: -${n.handle/2}px;
  border-radius: 1px;
  background: ${t.row};
  box-shadow: 0 0 0 1.5px ${t.selection};
}

${pe}
/* Generating's thumb: the board grid's shake at the board grid's pace, on the
   louder amplitude (Dor, 2026-09-22: "make the wiggle a bit stronger"). */
.${J} {
  animation: ${$e};
  transform-origin: 50% 50%;
}
@media (prefers-reduced-motion: reduce) {
  .${J} { animation: none; }
}
/* Generating's plate sweep, for a run with no input picture: the board's
   in-flight sweep (\`extend-image-shimmer\`, app-styles.ts) moves
   \`background-position\`, which repaints the plate on every frame of the run.
   This moves the same band (a layer twice the plate's width) the same distance
   in the same 3.5 s with a transform, which the compositor draws alone. */
@keyframes plnty-kit-gen-sweep {
  from { transform: translateX(50%); }
  to   { transform: translateX(-100%); }
}
.${ue} {
  animation: plnty-kit-gen-sweep 3.5s linear infinite;
}

/* Switch (master 905:10914, Dor's Enable Rotko drawing 779:73046). A control
   inside a bar, so it does not move; its three faces are the track, the knob
   and the arrow the knob wears, and the knob crosses to the label's far side
   while On. The faces are written as variables here so the knob and the arrow,
   drawn by ChromeSwitch, read them without a colour of their own (the master
   as redrawn on 2026-09-20 pm; the first cut had a colour/line track with a
   colour/panel knob, and the hover's words in colour/blue ink):
     Off    colour/pressed track, NO knob (the arrow sits on the track), a colour/line arrow, colour/ink words
     Hover  colour/blue at 25% over colour/field, colour/blue knob, a colour/selection arrow, colour/selection words
     On     colour/blue track, colour/selection knob, a colour/blue arrow, colour/blue ink words
   The Off words and the arrow's outline were colour/ink soft (#6C6962) until
   2026-09-22 (Dor, on the pill: "change #6C6962 to #1A1A19"); both are ink now. */
.${v} {
  --ks-knob: transparent;
  --ks-arrow: ${t.line};
  --ks-arrow-ink: ${t.ink};
  background: ${t.pressed};
  color: ${t.ink};
  transition:
    background ${e.hoverMs}ms ${e.hoverEase},
    color ${e.hoverMs}ms ${e.hoverEase};
}
/* data-landing: the knob catching Rothko as it flies home (rothko / user-facing,
   section 3, 2026-10-01), the Hover face for a beat with no pointer over it. */
.${v}:hover:not([aria-checked="true"]):not([aria-disabled="true"]),
.${v}[data-landing]:not([aria-checked="true"]):not([aria-disabled="true"]) {
  --ks-knob: ${t.blue};
  --ks-arrow: ${t.selection};
  --ks-arrow-ink: ${t.blueInk};
  /* One colour, not the master's two stacked fills, so the track can
     transition into it. */
  background: color-mix(in srgb, ${t.blue} 25%, ${t.field});
  color: ${t.selection};
}
.${v}[aria-checked="true"] {
  --ks-knob: ${t.selection};
  --ks-arrow: ${t.blue};
  --ks-arrow-ink: ${t.blueInk};
  background: ${t.blue};
  color: ${t.blueInk};
}
.${v}[aria-disabled="true"] { opacity: 0.5; }
.${W} {
  position: absolute;
  top: ${p.switchPad}px;
  left: ${p.switchPad}px;
  width: ${n.knob}px;
  height: ${n.knob}px;
  background: var(--ks-knob);
  transition:
    left ${e.hoverMs}ms ${e.hoverEase},
    background ${e.hoverMs}ms ${e.hoverEase};
}
.${v}[aria-checked="true"] .${W} {
  left: calc(100% - ${p.switchPad+n.knob}px);
}
/* The arrow's two paths read --ks-arrow / --ks-arrow-ink, and a custom
   property flips in one step, so the arrow used to turn blue on the first
   frame of a hover while the track, the knob and the words were still
   fading over the hover clock (Dor, 2026-09-22: "fix rothko so everything
   changes colour at once"). fill and stroke transition like any colour, so
   the arrow takes the same clock as the rest and the four change together. */
.${W} svg path {
  transition:
    fill ${e.hoverMs}ms ${e.hoverEase},
    stroke ${e.hoverMs}ms ${e.hoverEase};
}
/* The words keep the knob's room on the side the knob is on: pad + knob + gap
   before them while Off, after them while On. */
.${U} {
  padding-left: ${p.switchPad+n.knob+p.switchGap}px;
  padding-right: ${p.switchX}px;
  transition:
    padding ${e.hoverMs}ms ${e.hoverEase};
}
.${v}[aria-checked="true"] .${U} {
  padding-left: ${p.switchX}px;
  padding-right: ${p.switchPad+n.knob+p.switchGap}px;
}
@media (prefers-reduced-motion: reduce) {
  .${H}:hover:not([data-still]) { transform: none; }
  .${a}:hover { transform: none; }
  .${o}:hover { transform: none; }
  .${r}:hover { transform: none; }
  /* The X still comes up, without the rise: its opacity is not the animation's. */
  [${y}] .${o}:hover .${w},
  [${y}] .${o}.${A} .${w} { animation: none; }
  .${T}:hover > * { transform: none; }
  .${z}:hover { transform: none; }
  /* Reduced motion cuts the knob's crossing, not its colour: the four faces
     still change together, on the same clock as the track. */
  .${W} { transition: background ${e.hoverMs}ms ${e.hoverEase}; }
  .${U} { transition: none; }
}
/* ─── Rows ────────────────────────────────────────────────────────────── */
/* The wrapper. The pill inside it is the kit's Tool row (ToolPill wears
   KIT_ROW); this box carries only the row's BEHAVIOUR classes, which is why
   it paints nothing. ROW, ROW_DRAGGING and ROW_RETURNING are also read by
   Rothko's cursor performance, which clones a row. */
.${u} {
  position: relative;
  align-self: stretch;
  display: flex;
  align-items: stretch;
}
.${I} {
  flex: 1;
  min-width: 0;
}

/* ⚠ THE ROWS THAT ARE NOT THE KIT'S STILL READ THEIR FACE FROM HERE (2026-09-24).
   The column's pill is ToolPill and KIT_ROW gives it everything. Two lists
   build a row from ROW / ROW_INNER by hand and take their layout, ground, hover
   and drag slot from these rules: Edit mode's Tools shelf (EditModeShelf) and
   the fx tab's looks (FxShelf); the canvas tool picker was the third until it
   went (2026-09-28). The kit's rewrite (ddda1a1e3) removed the rules with the
   column's move to KIT_ROW, and the three lost them: the name fell under the
   icon and out of the row. They are back, scoped away from KIT_ROW so the
   column never sees them. :where() keeps each selector at its old weight, so
   the shelf's own skin (edit-mode-kit) still wins where it always did. */
.${I}:not(:where(.${a})) {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--sh-row-icon-gap, ${x.rowIconGap}px);
  padding: var(--sh-row-pad-y, ${x.rowPadY}px) var(--sh-row-pad-x, ${x.rowPadX}px);
  border: none;
  background: ${$.rowGround};
  color: ${$.ink};
  text-align: left;
  translate: 0 0;
  transition:
    background ${ne}ms ${c.easeOut},
    translate ${ne}ms ${c.easeOut},
    color 160ms ease,
    box-shadow 160ms ease;
}
.${u}:hover .${I}:not(:where(.${a})) {
  background: ${$.rowGroundHover};
  translate: 0 -${ye}px;
}
/* The slot a carried row leaves: a pressed hole under a hidden pill, holding
   its place so the list never reflows under the cursor. A kit row draws its
   own slot (KIT_ROW_EMPTY), the Tool row's, the Chain row's and the dominant
   row's alike. */
.${u}:not(:where(:has(.${a}, .${o}, .${r})))::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: var(--sh-row-radius, ${x.rowRadius*ce}px);
  corner-shape: squircle;
  background: transparent;
  box-shadow: none;
  pointer-events: none;
  z-index: 0;
  transition:
    background 420ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 420ms cubic-bezier(0.16, 1, 0.3, 1);
}
.${L}:not(:where(:has(.${a}, .${o}, .${r})))::before,
.${M}:not(:where(:has(.${a}, .${o}, .${r})))::before {
  background: rgba(0, 0, 0, 0.06);
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.14),
    inset 0 -1px 1px rgba(255, 255, 255, 0.4);
  transition: none;
}
.${L} .${I}:not(:where(.${a})),
.${M} .${I}:not(:where(.${a})) {
  visibility: hidden;
  transition: none;
}

/* ─── The favourite heart ─────────────────────────────────────────────── */
/* Master 703:36141: Chrome / heart small at 14, at the far end of the Lead (the
   name is Fill). Hidden at rest, up on hover (the fav-in rise), held up
   when the tool is saved, and the pop on a click. It sits INSIDE the pill's
   Lead (ToolPill § afterName), so it inherits nothing it should not: its ink
   is set by the panel (muted unsaved, ink saved). */
.${i} {
  width: ${n.iconSmall}px;
  height: ${n.iconSmall}px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  background: none;
  opacity: 0;
  flex-shrink: 0;
}
.${u}:hover:not(.${O}) .${i}:not(.${V}) {
  animation: plnty-shell-fav-in ${re}ms ${c.easeOut} forwards;
}
.${i}.${V} {
  animation: plnty-shell-fav-pop ${_e}ms ${c.easeOut} both;
}
@keyframes plnty-shell-fav-pop {
  0%   { opacity: 1; scale: 1; }
  28%  { opacity: 1; scale: 0.88; }
  56%  { opacity: 1; scale: 1.25; }
  100% { opacity: 1; scale: 1; }
}
@keyframes plnty-shell-fav-in {
  0%   { opacity: 0; translate: 0 ${Te}px; }
  55%  { opacity: 1; translate: 0 -${Ee}px; }
  100% { opacity: 1; translate: 0 0; }
}
.${O} .${i} { opacity: 1; }

/* ─── Badge / New at the far end (master 703:36152) ───────────────────── */
/* The row wears ONE trailing per state (Trailing: None | Heart | New): the
   badge at rest, the heart while the row is hovered, selected or saved,
   nothing in the hand. Whichever is off LEAVES the row (display none, never
   opacity 0), so the one that is on sits at the far end. A faded badge
   still took its width and the heart sat stranded left of it (Dor,
   2026-09-19). The row that wears a marker is the one with the
   data-tool-marker attribute (the panel sets it). */
.${R} { display: flex; }
.${u}:hover .${R},
.${O} .${R},
.${L} .${R},
.${M} .${R} { display: none; }
.${u}[data-tool-marker]:not(:hover):not(.${O}) .${i} { display: none; }
/* The dominant row keeps its heart OUT of the row at rest, not merely faded:
   the heart sits at the row's right end (3628:91, the operator's edit), beside
   the line under the name, and a faded heart would still take 14 of the width
   that line has. Up on hover, held up when saved, as on the Tool row. */
.${u}:not(:hover):not(.${O}) .${r} .${i}:not(.${V}) { display: none; }

/* ─── Drag ─────────────────────────────────────────────────────────────── */
/* The row in the hand is a clone (useToolRowGrab § arm) wearing
   KIT_ROW_DRAGGING and the +3 tilt; the row that stays wears KIT_ROW_EMPTY
   (set by the panel from phase), which is the kit's State=Empty slot. The
   heart never shows on either. */
.${L} .${i},
.${M} .${i} { opacity: 0; animation: none; }
/* The landing frames: see ROW_LANDING. The row takes its resting face at
   once, under the clone, instead of easing out of the hole in the open. */
.${F} .${a},
.${F} .${o},
.${F} .${r} { transition: none; }


/* ─── The top-bar pills ───────────────────────────────────────────────── */

/**
 * ⚠ THE RESTING GROUNDS LIVE HERE NOW, AND THEY USED TO BE INLINE STYLES.
 * That is not tidying: an inline style beats a class rule, so while pillBase
 * carried background the hover rule below could never apply and the pills had
 * NO hover state at all. It had been dead for as long as it had existed —
 * verified on the live shell, where the pill reported an inline
 * rgba(227, 226, 222, 0.8) sitting on top of it. Any resting appearance a
 * :hover rule needs to override has to be declared in the same cascade layer
 * as the override, which means here and not in the component.
 */
.${b} {
  background: ${$.chromeGround};
}
.${k} {
  background: ${$.chromeInverted};
}

.${b},
.${k} {
  translate: 0 0;
  transition:
    background ${c.downDuration} ${c.easeOut},
    translate ${ke}ms ${c.easeOut};
}
.${b}:hover,
.${k}:hover {
  animation: plnty-chrome-pill-lift ${be}ms ${c.easeOut} forwards;
}
/* Light goes darker, dark goes lighter — both toward the middle, so the two
   ends of the bar answer a pointer the same amount rather than the same way. */
.${b}:hover { background: ${$.chromeGroundHover}; }
.${k}:hover { background: ${$.chromeInvertedHover}; }

/**
 * A pill that is out of reach still hovers — it has to, or its tooltip never
 * opens — but it must not ANSWER. The hop and the ground shift are the two
 * things that say "this is live", so both are taken back and the pill sits
 * still under the pointer wearing its 0.45 (see ChromeAction.disabled).
 *
 * ⚠ THE :hover ON THE LEFT IS LOAD-BEARING, NOT DECORATION. Without it these
 * selectors weigh the same as the two rules above (one class + one qualifier
 * each) and win only on source order, which is the kind of thing a later edit
 * silently reverses. With it they are one qualifier heavier and cannot lose.
 */
.${b}[aria-disabled='true']:hover,
.${k}[aria-disabled='true']:hover {
  animation: none;
  translate: 0 0;
}
.${b}[aria-disabled='true']:hover { background: ${$.chromeGround}; }
.${k}[aria-disabled='true']:hover { background: ${$.chromeInverted}; }

@keyframes plnty-chrome-pill-lift {
  0%   { translate: 0 0; }
  /* Past the rest position, then back down onto it. The 55% is where the
     overshoot sits; the remaining 45% is the settle. */
  55%  { translate: 0 -${ae+fe}px; }
  100% { translate: 0 -${ae}px; }
}


/* ─── The selection row's corner star ─────────────────────────────────────
   A gold four-point star pinned to the top-right corner of one CHIP in the
   selection row, turning forever, saying that action is new. See SEL_SPARK_MS
   for the shape of the turn, and selection-bar-catalog's 'spark' for which chip
   wears it.

   ⚠ THE ANIMATION IS ON THE WRAPPER, NOT THE <svg>, AND THAT IS NOT TIDINESS.
   'rotate' as an individual transform property lands on an SVG element only in
   recent engines, and where it does not it fails silently — a star that simply
   never moves, on a surface nobody would think to check. An HTML span has taken
   it everywhere for years. The wrapper is also what carries the corner offsets,
   so putting the turn on it keeps position and rotation on one box instead of
   two that have to agree.

   ⚠ transform-origin IS NAMED THOUGH IT IS THE DEFAULT. The star is drawn
   centred in a square viewBox, so 50%/50% is its own middle; saying so is what
   stops a later transform-origin on an ancestor rule from quietly making it
   orbit rather than spin.

   ⚠ NO POINTER EVENTS. It sits INSIDE a button and overhangs onto the neighbour
   beside it, so a decoration that swallowed a press would break its own chip and
   the one next to it. That is the one way this can do harm. */
.${X} {
  animation: plnty-shell-sel-spark ${Ie}ms ease-in-out infinite;
  transform-origin: 50% 50%;
  pointer-events: none;
}

@keyframes plnty-shell-sel-spark {
  0%   { rotate: 0deg; }
  /* Each beat takes 40% of the loop and each rest 10%, so the star is moving
     four fifths of the time and still ends every beat at rest. */
  40%  { rotate: 90deg; }
  50%  { rotate: 90deg; }
  90%  { rotate: 180deg; }
  100% { rotate: 180deg; }
}

/* ─── The tools column's hover card ──────────────────────────────────────
   Its arrival (KIT_TIP_IN), and the drag line's wave (KIT_HINT_WAVE): each
   letter goes grey → ink → grey, starting its HINT_WAVE_STEP_MS after the one
   before, so the ink travels right. The line rests grey between passes. The
   letters are the line's aria-hidden spans; a screen reader reads the whole
   sentence from its hidden copy instead. */
.${oe} {
  animation: plnty-kit-tip-in ${e.hoverMs}ms ${e.hoverEase} both;
}
@keyframes plnty-kit-tip-in {
  from { opacity: 0; }
  to   { opacity: 1; }
}
.${Y} > [aria-hidden] {
  animation: plnty-kit-hint-wave ${ve}ms ease-in-out infinite;
  animation-delay: calc(var(--hint-i, 0) * ${ge}ms);
}
/* My chains' copy stays mounted while it fades out (it keeps its last chain
   for the fade) and says so with aria-hidden: nobody sees that line wave. */
[aria-hidden='true'] .${Y} > [aria-hidden] {
  animation-play-state: paused;
}
@keyframes plnty-kit-hint-wave {
  /* Up in 5% of the loop, down over the next 11: the band is a few letters
     of ink trailing off, not the whole line going dark at once. */
  0%, 16%, 100% { color: ${t.muted}; }
  5%            { color: ${t.ink}; }
}

@media (prefers-reduced-motion: reduce) {
  /* No rise, no bounce — but the heart still has to APPEAR, and its opacity
     came from the animation. Hand it back, or hovering a row under reduced
     motion offers no way to save it. */
  .${i} { animation: none; }
  .${u}:hover > .${i} { opacity: 1; }
  /* The star stops turning and stays put. It is a MARK first and a movement
     second, so taking the animation leaves the whole message on screen —
     unlike the heart above, which had nothing else making it visible. */
  .${X} { animation: none; }
  /* The card simply appears, and its drag line stays the grey it rests on. */
  .${oe} { animation: none; }
  .${Y} > [aria-hidden] { animation: none; }
}
`;let se=!1;function Me(){if(se||typeof document>"u")return;se=!0;const d=document.createElement("style");d.setAttribute("data-plnty","canvas-shell-behaviour"),d.textContent=Oe,document.head.appendChild(d)}export{M as A,D as B,y as C,z as D,B as E,r as F,P as G,i as H,V as I,xe as J,T as K,O as L,F as M,R as N,h as O,Ae as P,Se as Q,I as R,me as S,Le as T,X as U,q as V,H as a,Y as b,S as c,Q as d,l as e,G as f,a as g,w as h,Me as i,o as j,A as k,C as l,v as m,W as n,U as o,te as p,Z as q,J as r,ee as s,ue as t,k as u,b as v,oe as w,Ne as x,u as y,L as z};
