//! Probabilistic sampling scheduler is a corpus scheduler that feeds the fuzzer
//! with sampled item from the corpus.

use std::vec::Vec;
use hashbrown::HashSet;
use alloc::string::String;
use core::{marker::PhantomData, fmt::Debug};

use hashbrown::HashMap;
use libafl_bolts::rands::Rand;
use libafl_bolts::HasLen;
use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{
    corpus::{testcase::TestcaseMutationsMetadata, Corpus, CorpusId, HasTestcase, Testcase}, feedbacks::{cfg_prescience::{ControlFlowGraph, Reachability}, MapIndexesMetadata, MapNeighboursFeedbackMetadata, MapNoveltiesMetadata}, inputs::{Input, UsesInput}, schedulers::{Scheduler, TestcaseScore}, state::{HasCorpus, HasMetadata, HasNamedMetadata, HasRand, State, UsesState}, Error
};

/// A dummy TestcaseScore calculator
#[derive(Debug, Clone)]
pub struct UncoveredNeighboursMaxDistribution<I>
where
    I: Input,
{
    phantom: PhantomData<I>,
}

impl<S> TestcaseScore<S> for UncoveredNeighboursMaxDistribution<S::Input>
where
    S: HasMetadata + HasCorpus,
{
    fn compute(_state: &S, _entry: &mut Testcase<S::Input>) -> Result<f64, Error> {
        assert!(false, "I;m not supposed to be using this");
        Ok(0.0)
    }
}

/// A Probability sampling scheduler that bases testcase selection probabilities on the number of
/// reachable edges
pub type UncoveredNeighboursProbabilitySamplingScheduler<S> =
    ProbabilitySamplingScheduler<UncoveredNeighboursMaxDistribution<<S as UsesInput>::Input>, S>;

/// Conduct reservoir sampling (probabilistic sampling) over all corpus elements.
#[derive(Debug, Clone)]
pub struct ProbabilitySamplingScheduler<F, S>
where
    S: UsesInput,
{
    phantom: PhantomData<(F, S)>,
}

/// A state metadata holding a map of probability of corpus elements.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(
    any(not(feature = "serdeany_autoreg"), miri),
    allow(clippy::unsafe_derive_deserialize)
)] // for SerdeAny
pub struct ProbabilityMetadata {
    /// corpus index -> probability
    pub map: HashMap<CorpusId, f64>,
    /// total probability of all items in the map
    pub total_probability: f64,
    /// Do we need to recalculate the scores?
    pub needs_recalc: bool,
    /// The time that we last recalculated all the scores (in millis)
    pub last_recalc_time: u128,
    /// The amount of time the last recalc took
    pub last_recalc_duration: u128,
}

libafl_bolts::impl_serdeany!(ProbabilityMetadata);

impl ProbabilityMetadata {
    /// Creates a new [`struct@ProbabilityMetadata`]
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
            total_probability: 0.0,
            needs_recalc: false,
            last_recalc_time: 0,
            last_recalc_duration: 0,
        }
    }
}

impl Default for ProbabilityMetadata {
    fn default() -> Self {
        Self::new()
    }
}

struct ReachableBlocksResult {
    /// Number of corpus entries this reachability appears in
    frequency_for_reachability: HashMap<Reachability, usize>,
    /// Cumulative number of times corpus entries directly neighbouring (ie depth 1)
    /// this coverage map index have been mutated
    direct_neighbour_mutations_for_index: HashMap<usize, usize>,
    /// Lowest depth observed for the given index
    least_depth_for_index: HashMap<usize, usize>,
}


impl<F, S> ProbabilitySamplingScheduler<F, S>
where
    F: TestcaseScore<S>,
    S: HasCorpus + HasMetadata + HasNamedMetadata + HasRand,
    S::Input: HasLen,
{
    /// Creates a new [`struct@ProbabilitySamplingScheduler`]
    #[must_use]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    /// Calculate the score and store in `ProbabilityMetadata`
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::unused_self)]
    pub fn store_probability(&self, _state: &mut S, _idx: CorpusId) -> Result<(), Error> {
        assert!(false, "Should not use anymore");

        Ok(())
    }

    /// return a map of {(index, depth): frequency}, where frequency is the number of testcases
    /// with this index being reachable at the given depth
    fn recalculate_reachable_blocks(&self, state: &mut S) -> ReachableBlocksResult {
        let mut result = ReachableBlocksResult { 
            frequency_for_reachability: HashMap::new(),
            direct_neighbour_mutations_for_index: HashMap::new(),
            least_depth_for_index: HashMap::new(),
        };
        let start = Instant::now();

        let all_ids = state.corpus().ids().collect::<Vec<CorpusId>>();

        let full_neighbours_meta = state
            .metadata_mut::<MapNeighboursFeedbackMetadata>()
            .unwrap();

        let covered_blocks = full_neighbours_meta.covered_blocks.clone();
        let last_recalc_corpus_ids = full_neighbours_meta.corpus_ids_present_at_recalc.clone();
        full_neighbours_meta.corpus_ids_present_at_recalc = all_ids.clone();

        let mut recalcs = 0;
        for &idx in &all_ids {
            recalcs += 1;

            let tc = state.corpus().get(idx).unwrap().borrow();
            let covered_meta = tc.metadata::<MapIndexesMetadata>().unwrap();
            let covered_indexes = covered_meta.list.clone();
            let num_mutations = if let Ok(meta) = tc.metadata::<TestcaseMutationsMetadata>() {
                meta.num_mutations_executed        
            } else {
                0
            };
            drop(tc);

            let reachabilities = {
                let cfg_metadata = state.metadata_mut::<ControlFlowGraph>().unwrap();
                cfg_metadata.get_all_neighbours_full_depth(&covered_indexes, &covered_blocks)
            };

            if !last_recalc_corpus_ids.contains(&idx) {
                let tc = state.corpus().get(idx).unwrap().borrow();
                let novelties_meta = tc.metadata::<MapNoveltiesMetadata>().unwrap();
                let novelties = novelties_meta.list.clone();
                drop(tc);

                let full_neighbours_meta = state
                    .metadata_mut::<MapNeighboursFeedbackMetadata>()
                    .unwrap();

                for novelty in novelties {
                    full_neighbours_meta.reachable_blocks.remove(&novelty);
                }

                for reachability in &reachabilities {
                    full_neighbours_meta.reachable_blocks.insert(reachability.index);
                }
            }

            for reachability in reachabilities {
                // update reachability frequencies
                let new = if let Some(freq) = result.frequency_for_reachability.get(&reachability) {
                    freq + 1
                } else {
                    1
                };
                result.frequency_for_reachability.insert(reachability.clone(), new);

                // update number of mutations of direct neighbours (if appropriate)
                if reachability.depth == 1 {
                    let updated = if let Some(freq) = result.direct_neighbour_mutations_for_index.get(&reachability.index) {
                        freq + num_mutations
                    } else {
                        num_mutations
                    };
                    result.direct_neighbour_mutations_for_index.insert(reachability.index, updated);
                }

                // update least depth for index (if we beat the previous depth)
                if let Some(cur_min) = result.least_depth_for_index.get(&reachability.index) {
                    if reachability.depth < *cur_min { 
                        result.least_depth_for_index.insert(reachability.index, reachability.depth); 
                    }
                } else {
                    result.least_depth_for_index.insert(reachability.index, reachability.depth);
                }
            }
        }

        println!("Whew, recalced all neighbours for {recalcs} entries (out of a possible {}); took: {:.2?}", all_ids.len(), start.elapsed());

        result
    }

    /// Recalculate the probability of each testcase being selected for mutation
    pub fn recalc_all_probabilities(&self, state: &mut S) -> Result<(), Error> {
        let reachable_blocks_result = self.recalculate_reachable_blocks(state);

        let full_neighbours_meta = state
            .metadata::<MapNeighboursFeedbackMetadata>()
            .unwrap();
        let _reachable_all = full_neighbours_meta.reachable_blocks.clone();
        let covered_blocks = full_neighbours_meta.covered_blocks.clone();

        let mut total_score = 0.0;

        let ids = state.corpus().ids().collect::<Vec<CorpusId>>();
        let corpus_size = ids.len();
        let mut min_time = f64::MAX;

        // sort entries by time
        let mut time_ordered = vec![];
        let mut min_len = 99999999f64;
        let mut len_for_id = HashMap::new();
        for &id in &ids {
            let mut tc = state.corpus().get(id)?.borrow_mut();
            let len = tc.load_len(state.corpus()).unwrap() as f64;
            if len < min_len { min_len = len; }
            len_for_id.insert(id, len);
            let exec_time_ns = tc.exec_time().unwrap().as_nanos() as f64;
            if exec_time_ns < min_time { min_time = exec_time_ns; }
            time_ordered.push((id, exec_time_ns * len));
        }
        time_ordered.sort_by(|(_id, score1), (_id2, score2)| score1.partial_cmp(score2).unwrap());

        // The more this neighbour has been fuzzed, the less we'll prioritise it (maybe it's hard or infeasible)
        let backoff_weighting_for_direct_neighbour = {
            let mut weighting = HashMap::new();
            for (&index, &mutations) in &reachable_blocks_result.direct_neighbour_mutations_for_index {
                let decrements = mutations / 100;
                weighting.insert(index, 0.9999f64.powi(decrements as i32));
            }
            weighting
        };

        let mut favored_filled = HashSet::with_capacity(65536);
        let mut reachability_favored_ids = HashSet::new();
        let mut coverage_favored_ids = HashSet::new();
        let mut favored_edges = HashSet::new();
        let mut all_covered = HashSet::new();
        let mut neighbour_score_for_idx = HashMap::new();

        // greedily select a minimal subset of testcases that cover all neighbours (based on runtime)
        for &(entry, _runtime) in &time_ordered {
            let tc = state.corpus().get(entry)?.borrow();
            let idx_meta = tc.metadata::<MapIndexesMetadata>().unwrap();
            for &edge in &idx_meta.list { all_covered.insert(edge); }

            let mut neighbour_score = 0f64;
            let mut reachability_favored = false;

            let covered_indexes = idx_meta.list.clone();
            drop(tc);

            let reachabilities = {
                let cfg_metadata = state.metadata_mut::<ControlFlowGraph>().unwrap();
                cfg_metadata.get_all_neighbours_full_depth(&covered_indexes, &covered_blocks)
            };

            let tc = state.corpus().get(entry)?.borrow();
            let idx_meta = tc.metadata::<MapIndexesMetadata>().unwrap();
            for reachability in reachabilities {
                let freq = reachable_blocks_result.frequency_for_reachability.get(&reachability);
                if freq.is_none() { println!("frequency is none for {:?}", reachability); }
                let rarity = 1f64 / *freq.unwrap() as f64;
                let backoff_weighting = backoff_weighting_for_direct_neighbour.get(&reachability.direct_neighbour_ancestor_index);
                if backoff_weighting.is_none() { println!("backoff_weighting is none for {:?}", reachability.direct_neighbour_ancestor_index); }
                neighbour_score += backoff_weighting.unwrap() * rarity * 1f64 / reachability.depth as f64;
                // Make sure that we have entries that get as close as possible to all indexes
                if reachability.depth == reachable_blocks_result.least_depth_for_index[&reachability.index] {
                    reachability_favored |= favored_filled.insert(reachability.index);
                }
            }
            neighbour_score_for_idx.insert(entry, neighbour_score);
                
            let mut coverage_favored = false;
            for &edge in &idx_meta.list { 
                coverage_favored |= favored_edges.insert(edge); 
            }

            if reachability_favored {
                reachability_favored_ids.insert(entry);
            } else if coverage_favored {
                coverage_favored_ids.insert(entry);
            }
        }

        let skipped = all_covered.difference(&favored_edges).copied().collect::<Vec<usize>>();
        println!("Minimised the testset from {corpus_size} down to {} favored entries - and {} somewhat favored (favored edges: {}, missed out {} entries: {:?})", 
                 reachability_favored_ids.len(), coverage_favored_ids.len(),
                 favored_edges.len(), skipped.len(), skipped);

        let mut all_scores = vec![];
        for entry in ids {
            let tc = state.corpus().get(entry)?.borrow();
            let mut score = neighbour_score_for_idx[&entry];

            let exec_time_us = tc.exec_time().unwrap().as_nanos() as f64;
            let time_weighting = min_time / exec_time_us;
            score *= time_weighting;

            all_scores.push((entry, score));

            total_score += score;

            drop(tc);
            let meta = state
                .metadata_map_mut()
                .get_mut::<ProbabilityMetadata>()
                .unwrap();
            meta.map.insert(entry, score);
        }

        // all_scores.sort_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap());
        // println!("Scores: {:?}", all_scores);

        let meta = state
            .metadata_map_mut()
            .get_mut::<ProbabilityMetadata>()
            .unwrap();
        meta.total_probability = total_score;

        Ok(())
    }
}

impl<F, S> UsesState for ProbabilitySamplingScheduler<F, S>
where
    S: State + HasTestcase,
{
    type State = S;
}

impl<F, S> Scheduler for ProbabilitySamplingScheduler<F, S>
where
    F: TestcaseScore<S>,
    S: HasCorpus + HasNamedMetadata + HasMetadata + HasRand + HasTestcase + State,
    S::Input: HasLen,
{
    fn on_add(&mut self, state: &mut Self::State, idx: CorpusId) -> Result<(), Error> {
        let current_idx = *state.corpus().current();
        state
            .corpus()
            .get(idx)?
            .borrow_mut()
            .set_parent_id_optional(current_idx);

        if state.metadata_map().get::<ProbabilityMetadata>().is_none() {
            state.add_metadata(ProbabilityMetadata::new());
        }

        let prob_meta = state.metadata_map_mut().get_mut::<ProbabilityMetadata>().unwrap();
        prob_meta.needs_recalc = true;
        let avg = prob_meta.total_probability / prob_meta.map.len() as f64;
        prob_meta.map.insert(idx, avg);
        prob_meta.total_probability += avg;

        Ok(())
    }

    /// Gets the next entry
    #[allow(clippy::cast_precision_loss)]
    fn next(&mut self, state: &mut Self::State) -> Result<CorpusId, Error> {
        if state.corpus().count() == 0 {
            Err(Error::empty(String::from("No entries in corpus")))
        } else {
            const MAX_RAND: u64 = 1_000_000;
            let rand_prob: f64 = (state.rand_mut().below(MAX_RAND) as f64) / MAX_RAND as f64;

            let meta = state.metadata_map_mut().get_mut::<ProbabilityMetadata>().unwrap();
            if meta.needs_recalc {
                let ts_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                let time_since_recalc = ts_now - meta.last_recalc_time;
                let last_duration = meta.last_recalc_duration;
                // Don't spend more than 10% of the fuzzer time recalculating these stats - sure
                // this feels like we're not using the neighbours prescient power much at the start
                // of the campaign, but fuzzing campaigns last hours...
                if time_since_recalc >= (10 * last_duration)  {
                    println!("Last recalc took {last_duration}ms, now recalcing as it has been {time_since_recalc}");
                    let start = Instant::now();
                    self.recalculate_reachable_blocks(state);
                    self.recalc_all_probabilities(state).unwrap();

                    let meta = state.metadata_map_mut().get_mut::<ProbabilityMetadata>().unwrap();
                    meta.needs_recalc = false;
                    meta.last_recalc_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    meta.last_recalc_duration = start.elapsed().as_millis();
                }
            }

            let meta = state.metadata_map().get::<ProbabilityMetadata>().unwrap();
            let threshold = meta.total_probability * rand_prob;
            let mut k: f64 = 0.0;
            let mut ret = *meta.map.keys().last().unwrap();
            for (idx, prob) in &meta.map {
                k += prob;
                if k >= threshold {
                    ret = *idx;
                    break;
                }
            }
            self.set_current_scheduled(state, Some(ret))?;
            Ok(ret)
        }
    }
}

impl<F, S> Default for ProbabilitySamplingScheduler<F, S>
where
    F: TestcaseScore<S>,
    S: HasCorpus + HasNamedMetadata + HasMetadata + HasRand,
    S::Input: HasLen,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use core::{borrow::BorrowMut, marker::PhantomData};

    use libafl_bolts::rands::StdRand;

    use crate::{
        corpus::{Corpus, InMemoryCorpus, Testcase},
        feedbacks::ConstFeedback,
        inputs::{bytes::BytesInput, Input, UsesInput},
        schedulers::{ProbabilitySamplingScheduler, Scheduler, TestcaseScore},
        state::{HasCorpus, HasMetadata, StdState},
        Error,
    };

    const FACTOR: f64 = 1337.0;

    #[derive(Debug, Clone)]
    pub struct UniformDistribution<I>
    where
        I: Input,
    {
        phantom: PhantomData<I>,
    }

    impl<S> TestcaseScore<S> for UniformDistribution<S::Input>
    where
        S: HasMetadata + HasCorpus,
    {
        fn compute(_state: &S, _: &mut Testcase<S::Input>) -> Result<f64, Error> {
            Ok(FACTOR)
        }
    }

    pub type UniformProbabilitySamplingScheduler<S> =
        ProbabilitySamplingScheduler<UniformDistribution<<S as UsesInput>::Input>, S>;

    #[test]
    fn test_prob_sampling() {
        #[cfg(any(not(feature = "serdeany_autoreg"), miri))]
        unsafe {
            super::ProbabilityMetadata::register();
        }

        // the first 3 probabilities will be .69, .86, .44
        let rand = StdRand::with_seed(12);

        let mut scheduler = UniformProbabilitySamplingScheduler::new();

        let mut feedback = ConstFeedback::new(false);
        let mut objective = ConstFeedback::new(false);

        let mut corpus = InMemoryCorpus::new();
        let t1 = Testcase::with_filename(BytesInput::new(vec![0_u8; 4]), "1".into());
        let t2 = Testcase::with_filename(BytesInput::new(vec![1_u8; 4]), "2".into());

        let idx1 = corpus.add(t1).unwrap();
        let idx2 = corpus.add(t2).unwrap();

        let mut state = StdState::new(
            rand,
            corpus,
            InMemoryCorpus::new(),
            &mut feedback,
            &mut objective,
        )
        .unwrap();
        scheduler.on_add(state.borrow_mut(), idx1).unwrap();
        scheduler.on_add(state.borrow_mut(), idx2).unwrap();
        let next_idx1 = scheduler.next(&mut state).unwrap();
        let next_idx2 = scheduler.next(&mut state).unwrap();
        let next_idx3 = scheduler.next(&mut state).unwrap();
        assert_eq!(next_idx1, next_idx2);
        assert_ne!(next_idx1, next_idx3);
    }
}


