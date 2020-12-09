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
     - Given funds to `receiver` for `description` (up to 280 characters) and proposed `amount`
     - Finalize proposal
        When proposal has passed the require time, anyone can call to finalize it.
        If there is over 50% of council members voted "YES", proposal passes. Bond is returned to submitter and the action is executed.
        If over 50% voted "NO", proposal gets refused and bond kept.
        Otherwise (if not majority achieved): proposal fails and bond gets returned.
 - Only council members (or self) can call:
     - `vote` for a given proposal.
 - ``Finalize proposal can be called 
        - If this vote achieves >50% of council members saying "YES" - it executes action on success.
 - Upgradability with super majority vote of the council

Potentially, either council with vote can specify what amount they think make sense and then median of that  would be awarded.

Target audience for v1: [ToDo]

 - A person made a cool video about NEAR Wallet, development IDE, etc. They themself or anyone else can suggest to give them a bounty.
 - You saw really cool tweet bashing STATE bill - send that person a bounty (need them to create account though).
 - Someone contributed a small PR to one of NEAR libraries. One of maintainers can send them a bounty.
 - A person in NEAR Collective went beyond and above - another person in NEAR Collective sent them a grant.
 - Another GrantDAO applies for a grant to achieve their longer term goal via distributing to their guild members.

Even better: fork this code and create a more interesting ways to distribute.
Every guild can fork it and expand how this can be made more inclusive or more sophisticated.

Needs:
 - Nice frontend to visualize past and present proposals, creation of proposal, payouts, stats, etc.
 - This needs some form of notification service
 
V2 ideas:
 - Add support for other tokens in the "bank". Proposal can then specify either from whitelisted set of tokens.

# Development

## Deploy to TestNet

```bash
> near dev-deploy res/grandao.wasm
> near call dev-1607495280084-9068895 new '{"council": ["testmewell.testnet", "illia"], "bond": "1000000000000000000000000", "vote_period": "1800000000000"}' --accountId dev-1607495280084-9068895
> near view dev-1607495280084-9068895 get_num_proposals
> near call dev-1607495280084-9068895 add_proposal '{"proposal": {"target": "illia", "description": "test", "kind": {"Payout": { "amount": "1000000000000000000000000"}}}}' --accountId=illia --amount 1
> near view dev-1607495280084-9068895 get_proposal '{"id": 0}'
{
  status: 'Vote',
  proposer: 'illia',
  target: 'illia',
  description: 'test',
  kind: { Payout: { amount: '1000000000000000000000000' } },
  vote_period_end: 1607497778113967900,
  vote_yes: 0,
  vote_no: 0,
  votes: {}
}

> near view dev-1607495280084-9068895 get_proposals '{"from_index": 0, "limit": 1}'
> near call dev-1607495280084-9068895 vote '{"id": 0, "vote": "Yes"}' --accountId illia
```
