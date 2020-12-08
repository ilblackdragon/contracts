use near_sdk::{AccountId, Balance, env, Promise, near_bindgen, init};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedSet, UnorderedMap};
use near_lib::types::{Duration};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const MAX_DESCRIPTION_LENGTH: usize = 280;

#[derive(BorshSerialize, BorshDeserialize)]
enum Vote {
    Yes,
    No
}

#[derive(BorshSerialize, BorshDeserialize)]
enum ProposalKind {
    NewCouncil,
    RemoveCouncil,
    Payout {
        amount: Balance,
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct Proposal {
    proposer: AccountId,
    target: AccountId,
    description: String,
    kind: ProposalKind,
    vote_period_end: Duration,
    vote_yes: u64,
    vote_no: u64,
    votes: HashMap<AccountId, Vote>,
}

struct ProposalInput {
    target: AccountId,
    description: String,
    kind: ProposalKind,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
struct GrantDAO {
    bond: Balance,
    vote_period: Duration,
    council: UnorderedSet<AccountId>,
    proposals: UnorderedMap<u64, Proposal>,
    last_proposal_id: u64,
}

impl GrantDAO {
    #[init]
    pub fn new(council: Vec<AccountId>, bond: Balance, vote_period: Duration) -> Self {
        let mut dao = Self {
            bond,
            vote_period,
            council: UnorderedSet::new(b"c".to_vec()),
            proposals: UnorderedMap::new(b"p".to_vec()),
            last_proposal_id: 0
        };
        for account_id in council {
            dao.council.insert(&account_id);
        }
        dao
    }

    // #[payable]
    pub fn add_proposal(&mut self, proposal: ProposalInput) -> u64 {
        // TOOD: add also extra storage cost for the proposal itself.
        assert!(env::attached_deposit() >= self.bond, "Not enough deposit");
        assert!(proposal.description.len() < MAX_DESCRIPTION_LENGTH, "Description length is too long");
        let p = Proposal {
            proposer: env::predecessor_account_id(),
            target: proposal.target,
            description: proposal.description,
            kind: proposal.kind,
            vote_period_end: env::block_timestamp() + self.vote_period,
            vote_yes: 0,
            vote_no: 0,
            votes: HashMap::default(),
        };
        self.proposals.insert(&self.last_proposal_id, &p);
        self.last_proposal_id += 1;
        self.last_proposal_id - 1
    }

    pub fn get_council(&self) -> Vec<AccountId> {
        self.council.to_vec()
    }

    pub fn get_proposals(&self) -> Vec<(u64, Proposal)> {
        self.proposals.to_vec()
    }

    pub fn get_proposal(&self, id: u64) -> Proposal {
        self.proposals.get(&id).expect("Proposal not found")
    }

    pub fn vote(&mut self, id: u64, vote: Vote) {
        assert!(self.council.contains(&env::predecessor_account_id()), "Only council can vote");
        let mut proposal = self.proposals.get(&id).expect("No proposal with such id");
        if proposal.vote_period_end < env::block_timestamp() {
            env::log(b"Voting period expired, finalizing the proposal");
            let _ = self.finalize(id);
            return;
        }
        assert!(!proposal.votes.contains_key(&env::predecessor_account_id()), "Already voted");
        match vote {
            Vote::Yes => proposal.vote_yes += 1,
            Vote::No => proposal.vote_no += 1,
        }
        proposal.votes.insert(env::predecessor_account_id(), vote);
        self.proposals.insert(&id, &proposal);
    }

    pub fn finalize(&mut self, id: u64) {
        let proposal = self.proposals.get(&id).expect("No proposal with such id");
        assert!(proposal.vote_period_end < env::block_timestamp(), "Voting period has not expired");
        self.proposals.remove(&id);
        if proposal.vote_yes > proposal.vote_no {
            env::log(b"Vote succeeded");
            Promise::new(proposal.proposer).transfer(self.bond);
            match proposal.kind {
                ProposalKind::NewCouncil => {
                    self.council.insert(&proposal.target);
                },
                ProposalKind::RemoveCouncil => {
                    self.council.remove(&proposal.target);
                },
                ProposalKind::Payout { amount } => {
                    Promise::new(proposal.target).transfer(amount);
                },
            };
        } else if proposal.vote_no == 0 && proposal.vote_yes == 0 {
            // If no-one voted, let's return the bond.
            env::log(b"No vote");
            Promise::new(proposal.proposer).transfer(self.bond);
        } else {
            env::log(b"Vote failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{MockedBlockchain, testing_env};

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().finish());
        let mut dao = GrantDAO::new(vec![accounts(0), accounts(1)], 10, 1_000);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "add new member".to_string(),
            kind: ProposalKind::NewCouncil
        });
        assert_eq!(dao.get_proposals().len(), 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::Yes);
        assert_eq!(dao.get_proposal(id).vote_yes, 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).block_timestamp(1_001).finish());
        dao.finalize(id);
        assert_eq!(dao.get_council(), vec![accounts(0), accounts(1), accounts(2)]);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "give me money".to_string(),
            kind: ProposalKind::Payout { amount: 10 },
        });
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::No);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(1)).finish());
        dao.vote(id, Vote::Yes);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).block_timestamp(1_001).finish());
        dao.finalize(id);
    }

    #[test]
    #[should_panic]
    fn test_double_vote() {
        testing_env!(VMContextBuilder::new().finish());
        let mut dao = GrantDAO::new(vec![accounts(0), accounts(1)], 10, 1_000);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "add new member".to_string(),
            kind: ProposalKind::NewCouncil
        });
        assert_eq!(dao.get_proposals().len(), 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::Yes);
        dao.vote(id, Vote::Yes);
    }
}