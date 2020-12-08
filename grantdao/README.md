# GrantDAO

A simple version of a DAO to give out tips, bounties and grants.
Allows anyone to send a proposal to reward other people with funds and get a council to vote for it.

The major difference with Moloch DAO design is that this contract would receive its function via donation and council has equal rights.

Spec for v1:
 - Contract contains all the $NEAR in itself. It's initialized with it or receives later in form of donation.
 - There are council members: list of accounts that can vote for various activity. All council members have equal weight.
 - Next methods are available that can be called by anyone who attaches `bond` $NEAR (to prevent spam):
     - Add new council member
     - Remove council member
     - Given tip to `receiver` for `description` and proposed `tipAmount`
 - Only council members (or self) can call:
     - vote for a given proposal. If this vote achieves >50% of council members - it executes action on success or removes the proposal.

Potentially, either council with vote can specify how much tip they think make sense and then median of that  would be awarded.

We would launch this with 1000 $NEAR to just get a quick market feedback.

We can keep contract upgradable based on council decision as well to allow easily extend it going forward.


Target audience for v1: [ToDo]

 - A person made a cool video about NEAR Wallet, development IDE, etc. They themself or anyone else can suggest to give them a tip.
 - You saw really cool tweet bashing STATE bill - send that person a tip (need them to create account though).
 - Someone contributed a small PR to one of NEAR libraries. One of maintainers can send them a tip.
 - A person in NEAR Collective went beyond and above - another person in NEAR Collective sent them a tip.

Even better - let others fork this code and create a more interesting ways.

To expand on this ^:
I think we should emphasise that each guild can fork this and create their own version of this for their own Guild. For example Sandbox would def be down to do this. 

Could either be limited (TCR?) to a subset of accounts - members of that Guild or open to anyone. 

(Good addition for v2)