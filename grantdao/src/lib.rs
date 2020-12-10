use std::collections::HashMap;

use near_lib::types::{WrappedDuration, Duration, WrappedBalance};
use near_sdk::{AccountId, Balance, env, near_bindgen, Promise};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedSet, Vector};
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const MAX_DESCRIPTION_LENGTH: usize = 280;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
enum Vote {
    Yes,
    No
}

#[derive(BorshSerialize, BorshDeserialize, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum ProposalStatus {
    Vote,
    Success,
    Reject,
    Fail,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
enum ProposalKind {
    NewCouncil,
    RemoveCouncil,
    Payout {
        amount: WrappedBalance,
    },
    ChangeVotePeriod {
        vote_period: WrappedDuration
    },
    ChangeBond {
        bond: WrappedBalance,
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
struct Proposal {
    status: ProposalStatus,
    proposer: AccountId,
    target: AccountId,
    description: String,
    kind: ProposalKind,
    vote_period_end: Duration,
    vote_yes: u64,
    vote_no: u64,
    votes: HashMap<AccountId, Vote>,
}

impl Proposal {
    /// Compute new vote status given council size and current timestamp.
    pub fn vote_status(&self, num_council: u64) -> ProposalStatus {
        let majority = num_council / 2;
        if self.vote_yes > majority {
            ProposalStatus::Success
        } else if self.vote_no > majority {
            ProposalStatus::Reject
        } else if env::block_timestamp() > self.vote_period_end {
            ProposalStatus::Fail
        } else {
            ProposalStatus::Vote
        }
    }
}

#[derive(Serialize, Deserialize)]
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
    proposals: Vector<Proposal>,
}

impl Default for GrantDAO {
    fn default() -> Self {
        env::panic(b"GrantDAO should be initialized before usage")
    }
}

#[near_bindgen]
impl GrantDAO {
    #[init]
    pub fn new(council: Vec<AccountId>, bond: WrappedBalance, vote_period: WrappedDuration) -> Self {
        let mut dao = Self {
            bond: bond.into(),
            vote_period: vote_period.into(),
            council: UnorderedSet::new(b"c".to_vec()),
            proposals: Vector::new(b"p".to_vec()),
        };
        for account_id in council {
            dao.council.insert(&account_id);
        }
        dao
    }

    #[payable]
    pub fn add_proposal(&mut self, proposal: ProposalInput) -> u64 {
        // TOOD: add also extra storage cost for the proposal itself.
        assert!(env::attached_deposit() >= self.bond, "Not enough deposit");
        assert!(proposal.description.len() < MAX_DESCRIPTION_LENGTH, "Description length is too long");
        let p = Proposal {
            status: ProposalStatus::Vote,
            proposer: env::predecessor_account_id(),
            target: proposal.target,
            description: proposal.description,
            kind: proposal.kind,
            vote_period_end: env::block_timestamp() + self.vote_period,
            vote_yes: 0,
            vote_no: 0,
            votes: HashMap::default(),
        };
        self.proposals.push(&p);
        self.proposals.len() - 1
    }

    pub fn get_vote_period(&self) -> WrappedDuration {
        self.vote_period.into()
    }

    pub fn get_bond(&self) -> WrappedBalance {
        self.bond.into()
    }

    pub fn get_council(&self) -> Vec<AccountId> {
        self.council.to_vec()
    }

    pub fn get_num_proposals(&self) -> u64 {
        self.proposals.len()
    }

    pub fn get_proposals(&self, from_index: u64, limit: u64) -> Vec<Proposal> {
        (from_index..std::cmp::min(from_index + limit, self.proposals.len()))
            .map(|index| self.proposals.get(index).unwrap())
            .collect()
    }

    pub fn get_proposal(&self, id: u64) -> Proposal {
        self.proposals.get(id).expect("Proposal not found")
    }

    pub fn vote(&mut self, id: u64, vote: Vote) {
        assert!(self.council.contains(&env::predecessor_account_id()), "Only council can vote");
        let mut proposal = self.proposals.get(id).expect("No proposal with such id");
        assert!(proposal.status == ProposalStatus::Vote, "Proposal already finalized");
        if proposal.vote_period_end < env::block_timestamp() {
            env::log(b"Voting period expired, finalizing the proposal");
            self.finalize(id);
            return;
        }
        assert!(!proposal.votes.contains_key(&env::predecessor_account_id()), "Already voted");
        match vote {
            Vote::Yes => proposal.vote_yes += 1,
            Vote::No => proposal.vote_no += 1,
        }
        proposal.votes.insert(env::predecessor_account_id(), vote);
        self.proposals.replace(id, &proposal);
        // Finalize if this vote has achieved majority.
        if proposal.vote_status(self.council.len()) != ProposalStatus::Vote {
            self.finalize(id);
        }
    }

    pub fn finalize(&mut self, id: u64) {
        let mut proposal = self.proposals.get(id).expect("No proposal with such id");
        assert!(proposal.status == ProposalStatus::Vote, "Proposal already finalized");
        proposal.status = proposal.vote_status(self.council.len());
        match proposal.status {
            ProposalStatus::Success => {
                env::log(b"Vote succeeded");
                let target = proposal.target.clone();
                Promise::new(proposal.proposer.clone()).transfer(self.bond);
                match proposal.kind {
                    ProposalKind::NewCouncil => {
                        self.council.insert(&target);
                    },
                    ProposalKind::RemoveCouncil => {
                        self.council.remove(&target);
                    },
                    ProposalKind::Payout { amount } => {
                        Promise::new(target).transfer(amount.0);
                    },
                    ProposalKind::ChangeVotePeriod { vote_period } => {
                        self.vote_period = vote_period.into();
                    },
                    ProposalKind::ChangeBond { bond } => {
                        self.bond = bond.into();
                    }
                };
            },
            ProposalStatus::Reject => {
                env::log(b"Proposal rejected");
            }
            ProposalStatus::Fail => {
                // If no majority vote, let's return the bond.
                env::log(b"Proposal vote failed");
                Promise::new(proposal.proposer.clone()).transfer(self.bond);
            }
            ProposalStatus::Vote => env::panic(b"voting period has not expired and no majority vote yet")
        }
        self.proposals.replace(id, &proposal);
    }
}

#[cfg(test)]
mod tests {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{MockedBlockchain, testing_env};

    use super::*;

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().finish());
        let mut dao = GrantDAO::new(vec![accounts(0), accounts(1)], 10.into(), 1_000.into());

        assert_eq!(dao.get_bond(), 10.into());
        assert_eq!(dao.get_vote_period(), 1_000.into());

        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "add new member".to_string(),
            kind: ProposalKind::NewCouncil
        });
        assert_eq!(dao.get_num_proposals(), 1);
        assert_eq!(dao.get_proposals(0, 1).len(), 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::Yes);
        assert_eq!(dao.get_proposal(id).vote_yes, 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(1)).finish());
        dao.vote(id, Vote::Yes);
        assert_eq!(dao.get_council(), vec![accounts(0), accounts(1), accounts(2)]);

        // Pay out money for proposal. 2 votes yes vs 1 vote no.
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "give me money".to_string(),
            kind: ProposalKind::Payout { amount: 10.into() },
        });
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::No);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(1)).finish());
        dao.vote(id, Vote::Yes);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).finish());
        dao.vote(id, Vote::Yes);
        assert_eq!(dao.get_proposal(id).status, ProposalStatus::Success);

        // No vote for proposal.
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "give me more money".to_string(),
            kind: ProposalKind::Payout { amount: 10.into() },
        });
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(3)).block_timestamp(1_001).finish());
        dao.finalize(id);
        assert_eq!(dao.get_proposal(id).status, ProposalStatus::Fail);
    }

    #[test]
    fn test_single_council() {
        testing_env!(VMContextBuilder::new().finish());
        let mut dao = GrantDAO::new(vec![accounts(0)], 10.into(), 1_000.into());

        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(1),
            description: "add new member".to_string(),
            kind: ProposalKind::NewCouncil
        });
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::Yes);
        assert_eq!(dao.get_proposal(id).status, ProposalStatus::Success);
        assert_eq!(dao.get_council(), vec![accounts(0), accounts(1)]);
    }

    #[test]
    #[should_panic]
    fn test_double_vote() {
        testing_env!(VMContextBuilder::new().finish());
        let mut dao = GrantDAO::new(vec![accounts(0), accounts(1)], 10.into(), 1000.into());
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(10).finish());
        let id = dao.add_proposal(ProposalInput {
            target: accounts(2),
            description: "add new member".to_string(),
            kind: ProposalKind::NewCouncil
        });
        assert_eq!(dao.get_proposals(0, 1).len(), 1);
        testing_env!(VMContextBuilder::new().predecessor_account_id(accounts(0)).finish());
        dao.vote(id, Vote::Yes);
        dao.vote(id, Vote::Yes);
    }
}