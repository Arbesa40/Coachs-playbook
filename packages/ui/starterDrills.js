const d = (name, category, minutes, players, description, points, equipment = []) => ({ name, category, minutes, players, description, coaching_points: points, equipment });

export const STARTER_DRILLS = {
  football: [
    d("Rondo 4v1", "Rondo / possession", 10, "5 per grid", "Four attackers on the edges of a 10×10 m square keep the ball from one defender in the middle. Defender swaps in after a win or 10 passes.", "Open body shape; pass to the far foot; move to create two passing angles.", ["cone"]),
    d("Passing square", "Passing & receiving", 12, "8–12", "Players queue at four cones in a 15 m square, pass and follow to the next cone. Add a give-and-go and third-man run as the session goes on.", "Check away before receiving; first touch out of your feet; weight of pass.", ["cone"]),
    d("1v1 to end zones", "Dribbling & 1v1", 12, "Pairs", "Attacker tries to dribble into the defender's end zone in a 10×20 m channel. Swap roles each go.", "Attack the defender's front foot; change of pace after the move.", ["cone", "disc"]),
    d("Combination and finish", "Finishing", 15, "8–14 + GK", "Wall pass with a mannequin or server at the edge of the box, then a finish first time. Rotate sides.", "Finish across the keeper; arrive, don't wait; hit the target.", ["mannequin", "goal"]),
    d("6v4 pressing box", "Pressing", 15, "10", "Six try to keep the ball in a 25×25 m box; four press as a unit to win it and dribble out.", "Press on the trigger (bad touch, back pass); cut passing lanes; nearest presses, others cover.", ["cone"]),
  ],
  rugby: [
    d("2v1 and 3v2 overloads", "Handling & passing", 12, "Groups of 5", "Attackers run at defenders in a 15 m channel and must fix the defender before passing.", "Run straight to fix; hands up early; pass in front of the receiver.", ["cone"]),
    d("Tackle progression", "Contact & tackle technique", 15, "Pairs", "Kneeling to crouching to walking tackles on a partner or bag. Cheek to cheek, ring of steel, drive and roll.", "Eyes open; shoulder on target; leg drive; arms wrap and squeeze.", ["tackle_bag"]),
    d("Ruck clean-out on pads", "Breakdown / ruck", 12, "Groups of 4", "Ball carrier goes to ground; first two arrivals clean out pad holders past the ball.", "Low entry through the gate; bind; drive beyond the ball.", ["ruck_pad", "tackle_shield"]),
    d("Lineout pod lifting", "Lineout", 15, "Pods of 3 + thrower", "Front and back lifters practise a pod lift on calls; add a throw when timing is solid.", "Jumper sets, lifters drive through the hips; lock out and lower safely.", ["cone"]),
    d("Defensive line speed", "Defence", 12, "8–15", "Defenders come off a line together on the ball, outside man holds, inside pressure. Attack runs a simple shape.", "Line up together; communicate; squeeze from the inside.", ["cone"]),
  ],
  basketball: [
    d("Mikan drill", "Finishing at the rim", 5, "1 per basket", "Alternate right- and left-hand layups under the basket without the ball touching the floor.", "Soft hands; high release; use the backboard.", []),
    d("Three-man weave", "Passing", 8, "Groups of 3", "Three lines full court; pass and run behind the receiver; finish with a layup.", "Pass ahead of the runner; sprint wide lanes.", []),
    d("Shell drill", "Man-to-man defence", 15, "4v4", "Offence moves the ball around the perimeter; defence adjusts between on-ball, deny and help positions.", "Jump to the ball; see man and ball; talk.", ["cone"]),
    d("Form shooting", "Shooting", 10, "Pairs", "Close-range one-hand shots, stepping back as makes stack up.", "Balance; elbow in; hold the follow-through.", []),
  ],
  hockey: [
    d("Indian dribble gates", "Ball control & dribbling", 10, "Individual", "Players dribble through cone gates using Indian dribble, keeping the ball in front.", "Soft hands; head up between gates; keep the ball on the stick.", ["cone"]),
    d("Passing triangles", "Passing & receiving", 12, "Groups of 3", "Push passes around a 10 m triangle, then add a receive-on-the-move.", "Open stick to receive; transfer weight through the pass.", ["cone"]),
    d("Penalty corner: injection and trap", "Penalty corners", 15, "Injector, trapper, hitter", "Injector pushes from the back line; trapper stops at the top of the circle; hitter strikes on goal.", "Low, flat injection; clean trap; quick release.", ["goal"]),
    d("1v1 elimination", "Elimination skills", 12, "Pairs", "Attacker beats a defender using a pull or spin and shoots or passes to a target.", "Commit the defender; change direction late; accelerate away.", ["cone", "mini_goal"]),
  ],
  volleyball: [
    d("Pepper", "Warm-up", 8, "Pairs", "Pass, set, hit in a controlled pair sequence.", "Platform angle to target; ready position between contacts.", []),
    d("Serve to targets", "Serving", 12, "Whole team", "Serve to marked zones; score points for hitting the called zone.", "Consistent toss; contact behind the ball; aim deep corners.", ["target"]),
    d("Serve receive triangle", "Serve receive / passing", 15, "3 passers + server", "Passers receive serves and pass to a setter target.", "Feet to the ball; angle the platform; call the ball early.", ["target"]),
    d("Hitting lines", "Attacking / hitting", 15, "Hitters + setter", "Hitters approach and attack from the left and right with a live set.", "Full approach; high elbow; swing through the ball.", []),
  ],
  netball: [
    d("Landing and pivot ladder", "Footwork & landing", 10, "Individual", "Hop, land on one foot, pivot and pass back through the ladder.", "Land balanced; pivot on the landing foot; no stepping.", ["ladder"]),
    d("Two-ball passing", "Passing", 8, "Groups of 3", "Two balls in play at once around a triangle to build speed of release.", "Hands ready; quick release; accurate to the space.", ["cone"]),
    d("Centre pass breaks", "Centre pass attack", 15, "Centre + attackers", "WA, GA and GS break from the transverse line to receive the first pass.", "Timing of the drive; protect the space; strong hands on the catch.", ["cone"]),
    d("Shooting rotation", "Shooting", 15, "Shooters + feeders", "Shooters rotate through spots in the circle with feeders.", "Balance; ball on fingertips; follow through to the ring.", ["post"]),
  ],
};
