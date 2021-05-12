use std::sync::{Arc, Mutex};

use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use linefeed::Prompter;

use rink_core::{search::query, Context};

use crate::{config::Config, fmt::to_ansi_string};

pub struct RinkHelper {
    context: Arc<Mutex<Context>>,
    config: Config,
}
impl RinkHelper {
    pub fn new(context: Arc<Mutex<Context>>, config: Config) -> RinkHelper {
        RinkHelper { context, config }
    }
}

impl<Term: Terminal> Completer<Term> for RinkHelper {
    fn complete(
        &self,
        word: &str,
        _prompter: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let ctx = self.context.lock().unwrap();
        let reply = query(&ctx, word, 100);

        let results = reply
            .results
            .into_iter()
            .filter(|result| result.unit.as_ref().unwrap().starts_with(word))
            .take(10)
            .map(|result| Completion {
                display: Some(to_ansi_string(&self.config, &result)),
                completion: result.unit.unwrap(),
                suffix: linefeed::complete::Suffix::None,
            })
            .collect();

        Some(results)
    }
}
