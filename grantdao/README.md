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
     - Given funds to `receiver` for `description` and proposed `amount`
 - Only council members (or self) can call:
     - vote for a given proposal. If this vote achieves >50% of council members - it executes action on success or removes the proposal.

Potentially, either council with vote can specify what amount they think make sense and then median of that  would be awarded.

We would launch this with 1000 $NEAR to just get a quick market feedback.

We can keep contract upgradable based on council decision as well to allow easily extend it going forward.


Target audience for v1: [ToDo]

 - A person made a cool video about NEAR Wallet, development IDE, etc. They themself or anyone else can suggest to give them a bounty.
 - You saw really cool tweet bashing STATE bill - send that person a bounty (need them to create account though).
 - Someone contributed a small PR to one of NEAR libraries. One of maintainers can send them a bounty.
 - A person in NEAR Collective went beyond and above - another person in NEAR Collective sent them a grant.
 - Another GrantDAO applies for a grant to achieve their longer term goal via distributing to their guild members.

Even better: fork this code and create a more interesting ways to distribute.
Every guild can fork it and expand how this can be made more inclusive or more sophisticated.
