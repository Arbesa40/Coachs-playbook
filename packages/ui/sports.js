// Rules follow the international law books; anything marked `variation` can be switched off per team.

const pos = (code, name, group) => ({ code, name, group });

// ------------------------------------------------------------------ football
const football = {
  id: "football",
  formatOrder: ["11", "7", "5"],
  name: "Football",
  ball: "football",
  formats: {
    "11": {
      label: "11-a-side",
      periods: ["1st half", "2nd half"],
      periodMinutes: 45,
      extraPeriods: ["Extra time 1", "Extra time 2"],
      extraMinutes: 15,
      onField: 11,
      squad: 23,
      clock: "running",
      subs: { rolling: false, max: 5, note: "Usually 5 substitutions in 3 windows plus half-time; check your competition." },
    },
    "7": {
      label: "7-a-side (youth)",
      periods: ["1st half", "2nd half"],
      periodMinutes: 25,
      onField: 7,
      squad: 12,
      clock: "running",
      subs: { rolling: true, note: "Rolling substitutions." },
    },
    "5": {
      label: "5-a-side / Futsal",
      periods: ["1st half", "2nd half"],
      periodMinutes: 20,
      onField: 5,
      squad: 14,
      clock: "stoppable",
      subs: { rolling: true, note: "Unlimited flying substitutions." },
      timeouts: { perPeriod: 1, label: "Time-out (1 per half)" },
      teamFoulLimit: { count: 6, perPeriod: true, note: "From the 6th accumulated foul each half: direct free kick from 10 m." },
    },
  },
  positions: [
    pos("GK", "Goalkeeper", "Goalkeepers"),
    pos("RB", "Right back", "Defenders"), pos("CB", "Centre back", "Defenders"), pos("LB", "Left back", "Defenders"),
    pos("RWB", "Right wing-back", "Defenders"), pos("LWB", "Left wing-back", "Defenders"),
    pos("DM", "Defensive midfielder", "Midfielders"), pos("CM", "Central midfielder", "Midfielders"), pos("AM", "Attacking midfielder", "Midfielders"),
    pos("RM", "Right midfielder", "Midfielders"), pos("LM", "Left midfielder", "Midfielders"),
    pos("RW", "Right winger", "Forwards"), pos("LW", "Left winger", "Forwards"), pos("ST", "Striker", "Forwards"),
  ],
  groups: ["Goalkeepers", "Defenders", "Midfielders", "Forwards"],
  scoring: [
    { id: "goal", label: "Goal", points: 1, team: "us", player: true, methods: ["Open play", "Header", "Penalty", "Free kick", "Corner", "Counter-attack"] },
    { id: "own_goal_for", label: "Own goal (by them)", points: 1, team: "us" },
    { id: "goal_against", label: "Goal conceded", points: 1, team: "them", methods: ["Open play", "Header", "Penalty", "Free kick", "Corner", "Own goal"] },
  ],
  discipline: [
    { id: "yellow", label: "Yellow card", color: "#facc15", secondMeans: "red" },
    { id: "red_card", label: "Red card", color: "#ef4444", sendsOff: true },
  ],
  other: [
    { id: "penalty_awarded", label: "Penalty awarded", team: "either" },
    { id: "var_check", label: "VAR check", team: "either" },
    { id: "injury", label: "Injury stoppage", team: "us", player: true },
  ],
  drillCategories: ["Warm-up", "Rondo / possession", "Passing & receiving", "Dribbling & 1v1", "Finishing", "Crossing", "Pressing", "Defending", "Transition", "Build-up play", "Set pieces", "Small-sided games", "Goalkeeping", "Conditioning", "Cool-down"],
  equipment: ["cone", "disc", "pole", "mannequin", "mini_goal", "goal", "ladder", "hurdle", "flag", "rebounder"],
  lineKinds: ["run", "pass", "dribble", "shot", "move"],
  lineLabels: { dribble: "Dribble", shot: "Shot / cross" },
  tacticTypes: ["Corner (attacking)", "Corner (defending)", "Free kick (attacking)", "Free kick (defending)", "Throw-in", "Kick-off", "Penalty", "Build-up pattern", "Attacking pattern", "Pressing trigger", "Counter-attack", "Defensive shape", "Formation"],
};

// ------------------------------------------------------------------ rugby union
const rugbyPos15 = [
  pos("1", "Loosehead prop", "Front row"), pos("2", "Hooker", "Front row"), pos("3", "Tighthead prop", "Front row"),
  pos("4", "Lock (4)", "Second row"), pos("5", "Lock (5)", "Second row"),
  pos("6", "Blindside flanker", "Back row"), pos("7", "Openside flanker", "Back row"), pos("8", "Number 8", "Back row"),
  pos("9", "Scrum-half", "Half-backs"), pos("10", "Fly-half", "Half-backs"),
  pos("12", "Inside centre", "Centres"), pos("13", "Outside centre", "Centres"),
  pos("11", "Left wing", "Back three"), pos("14", "Right wing", "Back three"), pos("15", "Full-back", "Back three"),
];
const rugbyPos7 = [
  pos("P", "Prop", "Forwards"), pos("H", "Hooker", "Forwards"),
  pos("SH", "Scrum-half", "Backs"), pos("FH", "Fly-half", "Backs"), pos("C", "Centre", "Backs"), pos("W", "Wing", "Backs"),
];
const rugby = {
  id: "rugby",
  formatOrder: ["15", "7"],
  name: "Rugby union",
  ball: "rugby",
  formats: {
    "15": {
      label: "15s",
      periods: ["1st half", "2nd half"],
      periodMinutes: 40,
      extraPeriods: ["Extra time 1", "Extra time 2"],
      extraMinutes: 10,
      onField: 15,
      squad: 23,
      clock: "stoppable",
      sinBinMinutes: 10,
      positions: rugbyPos15,
      groups: ["Front row", "Second row", "Back row", "Half-backs", "Centres", "Back three"],
      subs: { rolling: false, max: 8, note: "Up to 8 replacements. Blood (15 min) and HIA (12 min) replacements are temporary." },
    },
    "7": {
      label: "Sevens",
      periods: ["1st half", "2nd half"],
      periodMinutes: 7,
      extraPeriods: ["Extra time (golden point)"],
      extraMinutes: 5,
      onField: 7,
      squad: 13,
      clock: "stoppable",
      sinBinMinutes: 2,
      positions: rugbyPos7,
      groups: ["Forwards", "Backs"],
      subs: { rolling: false, max: 5, note: "Up to 5 replacements; finals are often 10-minute halves." },
    },
  },
  scoring: [
    { id: "try", label: "Try", points: 5, team: "us", player: true },
    { id: "conversion", label: "Conversion", points: 2, team: "us", player: true },
    { id: "penalty_goal", label: "Penalty goal", points: 3, team: "us", player: true },
    { id: "drop_goal", label: "Drop goal", points: 3, team: "us", player: true },
    { id: "penalty_try", label: "Penalty try", points: 7, team: "us" },
    { id: "try_against", label: "Try conceded", points: 5, team: "them" },
    { id: "conversion_against", label: "Conversion conceded", points: 2, team: "them" },
    { id: "penalty_goal_against", label: "Penalty goal conceded", points: 3, team: "them" },
    { id: "drop_goal_against", label: "Drop goal conceded", points: 3, team: "them" },
    { id: "penalty_try_against", label: "Penalty try conceded", points: 7, team: "them" },
  ],
  discipline: [
    { id: "yellow", label: "Yellow card (sin bin)", color: "#facc15", sinBin: true },
    { id: "red20", label: "20-minute red card", color: "#f97316", sendsOff: true, replaceAfterMinutes: 20, variation: true },
    { id: "red_card", label: "Red card", color: "#ef4444", sendsOff: true },
    { id: "penalty_conceded", label: "Penalty conceded", color: "#94a3b8", team: "either", count: true },
  ],
  other: [
    { id: "kick_missed", label: "Kick missed", team: "us", player: true },
    { id: "blood_off", label: "Blood replacement (15 min)", team: "us", player: true, temporaryMinutes: 15 },
    { id: "hia_off", label: "HIA (12 min)", team: "us", player: true, temporaryMinutes: 12 },
    { id: "tmo_review", label: "TMO review", team: "either" },
  ],
  drillCategories: ["Warm-up", "Handling & passing", "Catch & pass under pressure", "Running lines", "Contact & tackle technique", "Breakdown / ruck", "Defence", "Attack shape", "Scrum", "Lineout", "Kicking game", "Kick-off & restarts", "Evasion & footwork", "Conditioning", "Cool-down"],
  equipment: ["cone", "disc", "tackle_bag", "tackle_shield", "ruck_pad", "kicking_tee", "pole", "flag", "ladder"],
  lineKinds: ["run", "pass", "dribble", "kick", "move"],
  lineLabels: { dribble: "Carry", kick: "Kick" },
  tacticTypes: ["Backline move", "Lineout move", "Scrum play", "Kick-off (receiving)", "Kick-off (kicking)", "Exit strategy", "Attack phase shape", "Defensive system", "Penalty / tap play", "Maul"],
};

// ------------------------------------------------------------------ basketball
const basketball = {
  id: "basketball",
  formatOrder: ["5x5", "3x3"],
  name: "Basketball",
  ball: "basketball",
  formats: {
    "5x5": {
      label: "5x5 (FIBA)",
      periods: ["Q1", "Q2", "Q3", "Q4"],
      periodMinutes: 10,
      extraPeriods: ["OT1", "OT2", "OT3"],
      extraMinutes: 5,
      onField: 5,
      squad: 12,
      clock: "stoppable",
      personalFoulLimit: 5,
      teamFoulLimit: { count: 5, perPeriod: true, note: "From the 5th team foul in a quarter, fouls give free throws." },
      timeouts: { firstHalf: 2, secondHalf: 3, overtime: 1, label: "Time-out (2 in 1st half, 3 in 2nd half, 1 per OT)" },
      subs: { rolling: true, note: "Unlimited substitutions on dead balls." },
    },
    "3x3": {
      label: "3x3",
      periods: ["Game"],
      periodMinutes: 10,
      extraPeriods: ["Overtime (first to 2)"],
      extraMinutes: 0,
      onField: 3,
      squad: 4,
      clock: "stoppable",
      winAt: 21,
      teamFoulLimit: { count: 7, perPeriod: false, note: "Team fouls 7-9: two free throws; 10+: two free throws and possession." },
      timeouts: { perGame: 1, label: "Time-out (1 per team)" },
      subs: { rolling: true, note: "Substitutions on dead balls." },
      scoring: [
        { id: "pt1", label: "1 pt (inside arc)", points: 1, team: "us", player: true },
        { id: "pt2", label: "2 pt (beyond arc)", points: 2, team: "us", player: true },
        { id: "ft", label: "Free throw", points: 1, team: "us", player: true },
        { id: "pts_against_1", label: "1 pt conceded", points: 1, team: "them" },
        { id: "pts_against_2", label: "2 pt conceded", points: 2, team: "them" },
      ],
    },
  },
  positions: [
    pos("PG", "Point guard", "Guards"), pos("SG", "Shooting guard", "Guards"),
    pos("SF", "Small forward", "Forwards"), pos("PF", "Power forward", "Forwards"),
    pos("C", "Centre", "Centres"),
  ],
  groups: ["Guards", "Forwards", "Centres"],
  scoring: [
    { id: "pt2", label: "2 pts", points: 2, team: "us", player: true },
    { id: "pt3", label: "3 pts", points: 3, team: "us", player: true },
    { id: "ft", label: "Free throw", points: 1, team: "us", player: true },
    { id: "pts_against_1", label: "1 pt conceded", points: 1, team: "them" },
    { id: "pts_against_2", label: "2 pts conceded", points: 2, team: "them" },
    { id: "pts_against_3", label: "3 pts conceded", points: 3, team: "them" },
  ],
  discipline: [
    { id: "personal_foul", label: "Personal foul", color: "#94a3b8", foul: true },
    { id: "technical_foul", label: "Technical foul", color: "#facc15", foul: true },
    { id: "unsportsmanlike_foul", label: "Unsportsmanlike foul", color: "#f97316", foul: true, twoMeansOut: true },
    { id: "disqualifying_foul", label: "Disqualifying foul", color: "#ef4444", sendsOff: true },
    { id: "foul_against", label: "Foul by opponent", color: "#64748b", team: "them", foul: true },
  ],
  other: [
    { id: "miss2", label: "2 pt miss", team: "us", player: true },
    { id: "miss3", label: "3 pt miss", team: "us", player: true },
    { id: "rebound", label: "Rebound", team: "us", player: true },
    { id: "assist", label: "Assist", team: "us", player: true },
    { id: "steal", label: "Steal", team: "us", player: true },
    { id: "turnover", label: "Turnover", team: "us", player: true },
  ],
  drillCategories: ["Warm-up", "Ball handling", "Passing", "Shooting", "Finishing at the rim", "Rebounding", "1v1 / closeouts", "Pick and roll", "Motion offence", "Man-to-man defence", "Zone defence", "Press & press break", "Transition", "Conditioning", "Cool-down"],
  equipment: ["cone", "disc", "chair", "pole", "ladder"],
  lineKinds: ["run", "pass", "dribble", "screen", "move"],
  lineLabels: { dribble: "Dribble", screen: "Screen" },
  tacticTypes: ["Half-court set", "Baseline out of bounds (BLOB)", "Sideline out of bounds (SLOB)", "Press break", "Zone offence", "Man-to-man defence", "Zone defence", "Full-court press", "End of quarter play", "Jump ball"],
};

// ------------------------------------------------------------------ hockey (field)
const hockey = {
  id: "hockey",
  formatOrder: ["11", "5s"],
  name: "Field hockey",
  ball: "hockey",
  formats: {
    "11": {
      label: "11-a-side",
      periods: ["Q1", "Q2", "Q3", "Q4"],
      periodMinutes: 15,
      onField: 11,
      squad: 18,
      clock: "stoppable",
      subs: { rolling: true, note: "Unlimited rolling substitutions (not during a penalty corner)." },
    },
    "5s": {
      label: "Hockey5s",
      periods: ["1st half", "2nd half"],
      periodMinutes: 10,
      onField: 5,
      squad: 9,
      clock: "stoppable",
      subs: { rolling: true, note: "Rolling substitutions; period length varies by competition." },
    },
  },
  positions: [
    pos("GK", "Goalkeeper", "Goalkeepers"),
    pos("DEF", "Defender", "Defenders"), pos("SW", "Sweeper", "Defenders"),
    pos("MID", "Midfielder", "Midfielders"),
    pos("FWD", "Forward", "Forwards"),
  ],
  groups: ["Goalkeepers", "Defenders", "Midfielders", "Forwards"],
  scoring: [
    { id: "goal", label: "Field goal", points: 1, team: "us", player: true },
    { id: "pc_goal", label: "Penalty corner goal", points: 1, team: "us", player: true },
    { id: "stroke_goal", label: "Penalty stroke goal", points: 1, team: "us", player: true },
    { id: "goal_against", label: "Goal conceded", points: 1, team: "them", methods: ["Field goal", "Penalty corner", "Penalty stroke"] },
  ],
  discipline: [
    { id: "green", label: "Green card (2 min)", color: "#22c55e", sinBin: true, minutes: 2 },
    { id: "yellow", label: "Yellow card (5 min)", color: "#facc15", sinBin: true, minutes: 5 },
    { id: "yellow10", label: "Yellow card (10 min)", color: "#eab308", sinBin: true, minutes: 10 },
    { id: "red_card", label: "Red card", color: "#ef4444", sendsOff: true },
  ],
  other: [
    { id: "pc_for", label: "Penalty corner won", team: "us" },
    { id: "pc_against", label: "Penalty corner conceded", team: "them" },
    { id: "stroke_for", label: "Penalty stroke won", team: "us" },
    { id: "circle_entry", label: "Circle entry", team: "us" },
  ],
  drillCategories: ["Warm-up", "Ball control & dribbling", "Passing & receiving", "Elimination skills", "Shooting & deflections", "Penalty corners", "Defending & tackling", "Press", "Outletting / build-up", "Transition", "Small-sided games", "Goalkeeping", "Conditioning", "Cool-down"],
  equipment: ["cone", "disc", "mini_goal", "goal", "pole", "flag", "ladder"],
  lineKinds: ["run", "pass", "dribble", "shot", "move"],
  lineLabels: { dribble: "Carry", shot: "Shot / hit" },
  tacticTypes: ["Penalty corner (attacking)", "Penalty corner (defending)", "Free hit (attacking)", "Free hit (defending)", "Outlet / build-up", "Press", "Defensive structure", "Counter-attack", "Long corner", "Formation"],
};

// ------------------------------------------------------------------ volleyball
const volleyball = {
  id: "volleyball",
  formatOrder: ["indoor", "beach"],
  name: "Volleyball",
  ball: "volleyball",
  formats: {
    indoor: {
      label: "Indoor (6s)",
      periods: ["Set 1", "Set 2", "Set 3", "Set 4", "Set 5"],
      onField: 6,
      squad: 14,
      clock: "none",
      sets: { bestOf: 5, pointsToWin: 25, decidingSetPoints: 15, winBy: 2 },
      timeouts: { perSet: 2, label: "Time-out (2 per set)" },
      subs: { rolling: false, maxPerSet: 6, note: "6 substitutions per set; libero replacements don't count." },
      positions: [
        pos("S", "Setter", "Setters"), pos("OH", "Outside hitter", "Pin hitters"), pos("OPP", "Opposite", "Pin hitters"),
        pos("MB", "Middle blocker", "Middles"), pos("L", "Libero", "Back-court"), pos("DS", "Defensive specialist", "Back-court"),
      ],
      groups: ["Setters", "Pin hitters", "Middles", "Back-court"],
    },
    beach: {
      label: "Beach (2s)",
      periods: ["Set 1", "Set 2", "Set 3"],
      onField: 2,
      squad: 2,
      clock: "none",
      sets: { bestOf: 3, pointsToWin: 21, decidingSetPoints: 15, winBy: 2 },
      timeouts: { perSet: 1, label: "Time-out (1 per set)" },
      subs: { rolling: false, maxPerSet: 0, note: "No substitutions." },
      positions: [pos("BL", "Blocker", "Players"), pos("DEF", "Defender", "Players")],
      groups: ["Players"],
    },
  },
  scoring: [
    { id: "point_won", label: "Point won", points: 1, team: "us", player: false, methods: ["Kill", "Ace", "Block", "Opponent error"] },
    { id: "point_lost", label: "Point lost", points: 1, team: "them", methods: ["Opponent kill", "Opponent ace", "Opponent block", "Our error"] },
  ],
  discipline: [
    { id: "warning", label: "Warning (yellow)", color: "#facc15" },
    { id: "penalty_card", label: "Penalty (red): point to opponent", color: "#ef4444", givesPointTo: "them" },
    { id: "expulsion", label: "Expulsion (rest of set)", color: "#b91c1c" },
    { id: "disqualification", label: "Disqualification", color: "#7f1d1d", sendsOff: true },
  ],
  other: [],
  drillCategories: ["Warm-up", "Serving", "Serve receive / passing", "Setting", "Attacking / hitting", "Blocking", "Digging & defence", "Transition", "Rotations & systems", "Game-like drills", "Conditioning", "Cool-down"],
  equipment: ["cone", "target", "disc", "chair", "ball_cart"],
  lineKinds: ["run", "pass", "shot", "move"],
  lineLabels: { pass: "Pass / set", shot: "Attack / serve" },
  tacticTypes: ["Serve receive rotation", "Offensive system (5-1 / 6-2)", "Attack combination", "Blocking scheme", "Defensive system", "Serve strategy", "Transition play"],
};

// ------------------------------------------------------------------ netball
const netball = {
  id: "netball",
  formatOrder: ["standard"],
  name: "Netball",
  ball: "netball",
  formats: {
    standard: {
      label: "7-a-side",
      periods: ["Q1", "Q2", "Q3", "Q4"],
      periodMinutes: 15,
      extraPeriods: ["Extra time 1", "Extra time 2"],
      extraMinutes: 5,
      onField: 7,
      squad: 12,
      clock: "stoppable",
      subs: { rolling: false, note: "Substitutions at intervals, or during play for injury or illness." },
      suspensionMinutes: 2,
    },
  },
  positions: [
    pos("GS", "Goal shooter", "Shooters"), pos("GA", "Goal attack", "Shooters"),
    pos("WA", "Wing attack", "Midcourt"), pos("C", "Centre", "Midcourt"), pos("WD", "Wing defence", "Midcourt"),
    pos("GD", "Goal defence", "Defenders"), pos("GK", "Goal keeper", "Defenders"),
  ],
  groups: ["Shooters", "Midcourt", "Defenders"],
  scoring: [
    { id: "goal", label: "Goal", points: 1, team: "us", player: true, onlyPositions: ["GS", "GA"] },
    { id: "super_shot", label: "Super Shot (2)", points: 2, team: "us", player: true, onlyPositions: ["GS", "GA"], variation: true },
    { id: "goal_against", label: "Goal conceded", points: 1, team: "them" },
    { id: "super_shot_against", label: "Super Shot conceded (2)", points: 2, team: "them", variation: true },
  ],
  discipline: [
    { id: "caution", label: "Caution", color: "#facc15" },
    { id: "warning", label: "Warning", color: "#f97316" },
    { id: "suspension", label: "Suspension (2 min)", color: "#ef4444", sinBin: true, minutes: 2 },
    { id: "ordered_off", label: "Ordered off", color: "#7f1d1d", sendsOff: true },
  ],
  other: [
    { id: "missed_shot", label: "Missed shot", team: "us", player: true },
    { id: "intercept", label: "Intercept", team: "us", player: true },
    { id: "turnover", label: "Turnover", team: "us", player: true },
    { id: "centre_pass", label: "Centre pass", team: "either" },
  ],
  drillCategories: ["Warm-up", "Footwork & landing", "Passing", "Driving & leads", "Shooting", "Circle feeding", "Centre pass attack", "Defence (1v1)", "Zone defence", "Rebounding", "Transition", "Conditioning", "Cool-down"],
  equipment: ["cone", "disc", "post", "ladder", "hurdle"],
  lineKinds: ["run", "pass", "shot", "move"],
  lineLabels: { shot: "Shot" },
  tacticTypes: ["Centre pass (attacking)", "Centre pass (defending)", "Backline throw-in", "Sideline throw-in", "Circle edge play", "Zone defence", "Player-to-player defence", "Transition play"],
};

export const SPORTS = { football, rugby, basketball, hockey, volleyball, netball };
export const SPORT_IDS = Object.keys(SPORTS);

export function defaultFormat(sportId) {
  return SPORTS[sportId]?.formatOrder?.[0] ?? null;
}

// [[id, label], ...] in display order.
export function formatsOf(sportId) {
  const s = SPORTS[sportId];
  return (s?.formatOrder ?? []).map((id) => [id, s.formats[id].label]);
}

// Everything for one team: sport + format merged, with sensible defaults.
export function sportConfig(sportId = "football", formatId = null) {
  const sport = SPORTS[sportId] ?? football;
  const fid = sport.formats[formatId] ? formatId : defaultFormat(sport.id);
  const f = sport.formats[fid];
  return {
    ...sport,
    ...f,
    formatId: fid,
    formatLabel: f.label,
    positions: f.positions ?? sport.positions,
    groups: f.groups ?? sport.groups,
    scoring: f.scoring ?? sport.scoring,
    discipline: (f.discipline ?? sport.discipline).map((d) =>
      d.sinBin && !d.minutes ? { ...d, minutes: f.sinBinMinutes ?? f.suspensionMinutes ?? 10, label: d.label.replace("(sin bin)", `(${f.sinBinMinutes ?? 10} min)`) } : d
    ),
    other: f.other ?? sport.other,
  };
}

export function teamConfig(team) {
  return sportConfig(team?.sport, team?.format);
}

// Position group for a player's position code (or name).
export function groupOf(cfg, position) {
  if (!position) return "Unassigned";
  const v = String(position).trim().toLowerCase();
  const p = cfg.positions.find((x) => x.code.toLowerCase() === v || x.name.toLowerCase() === v);
  return p?.group ?? "Other";
}

// Players grouped by position group, in the sport's group order.
export function groupPlayers(cfg, players) {
  const order = [...cfg.groups, "Other", "Unassigned"];
  const map = new Map(order.map((g) => [g, []]));
  for (const p of players) map.get(groupOf(cfg, p.position))?.push(p) ?? map.get("Other").push(p);
  return order.map((g) => [g, map.get(g)]).filter(([, list]) => list.length);
}

export const LINE_KINDS = {
  run: { label: "Run", dash: "", wave: false, head: "arrow" },
  pass: { label: "Pass", dash: "1.6 1.1", wave: false, head: "arrow" },
  dribble: { label: "Dribble", dash: "", wave: true, head: "arrow" },
  shot: { label: "Shot", dash: "0.4 0.8", wave: false, head: "arrow", thick: 1.5 },
  kick: { label: "Kick", dash: "0.4 0.8", wave: false, head: "arrow", curve: true },
  screen: { label: "Screen", dash: "", wave: false, head: "bar" },
  move: { label: "Movement", dash: "", wave: false, head: "arrow", thin: true },
};

export function lineLabel(cfg, kind) {
  return cfg.lineLabels?.[kind] ?? LINE_KINDS[kind]?.label ?? kind;
}

export const EQUIPMENT = {
  cone: "Cone",
  disc: "Disc marker",
  pole: "Pole",
  mannequin: "Mannequin",
  mini_goal: "Mini goal",
  goal: "Goal",
  ladder: "Agility ladder",
  hurdle: "Hurdle",
  flag: "Flag",
  rebounder: "Rebounder",
  tackle_bag: "Tackle bag",
  tackle_shield: "Tackle shield",
  ruck_pad: "Ruck pad",
  kicking_tee: "Kicking tee",
  chair: "Chair / screen dummy",
  target: "Target zone",
  ball_cart: "Ball cart",
  post: "Goal post",
};
