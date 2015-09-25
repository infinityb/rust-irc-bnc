use std::cmp;
use std::marker::PhantomData;
use std::collections::{BTreeSet, BTreeMap, Bound};

use time::SteadyTime;
use irc::{IrcMsg, OSCaseMapping};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum QueryBuffer {
    Any,
    Server,
    Target(BufferTarget),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum QueryDirection {
    Oldest,
    Later(SteadyTime),
    Earlier(SteadyTime),
    Newest,
}

pub struct Query {
    buffer: QueryBuffer,
    direction: QueryDirection,
}

impl Query {
    pub fn any() -> Query {
        Query {
            buffer: QueryBuffer::Any,
            direction: QueryDirection::Newest,
        }
    }

    pub fn server() -> Query {
        Query {
            buffer: QueryBuffer::Server,
            direction: QueryDirection::Newest,
        }
    }

    pub fn target(buffer: BufferTarget) -> Query {
        Query {
            buffer: QueryBuffer::Target(buffer),
            direction: QueryDirection::Newest,
        }
    }

    pub fn oldest(self) -> Query {
        Query {
            buffer: self.buffer,
            direction: QueryDirection::Oldest,
        }
    }

    pub fn later(self, when: SteadyTime) -> Query {
        Query {
            buffer: self.buffer,
            direction: QueryDirection::Later(when),
        }
    }

    pub fn earlier(self, when: SteadyTime) -> Query {
        Query {
            buffer: self.buffer,
            direction: QueryDirection::Earlier(when),
        }
    }

    pub fn newest(self) -> Query {
        Query {
            buffer: self.buffer,
            direction: QueryDirection::Newest,
        }
    }
}

pub enum QueryError {
    //
}

pub struct QueryResult {
    next: Query,
    messages: Vec<IrcMsg>,
}

pub struct Server {
    case_map: Box<OSCaseMapping>,
    sub_buffers: BTreeSet<(QueryBuffer, SteadyTime)>,
    buffer: BTreeMap<SteadyTime, IrcMsg>,
}

fn query_helper<'a, I>(iter: I, buffer: &BTreeMap<SteadyTime, IrcMsg>, limit: usize) -> Option<(SteadyTime, Vec<IrcMsg>)>
    where I: Iterator<Item=&'a (QueryBuffer, SteadyTime)>
{
    let mut last_key = None;
    let mut results = Vec::with_capacity(limit);
    for &(_, when) in iter {
        last_key = Some(when);
        results.push(buffer[&when].clone());
    }
    last_key.map(|lk| (lk, results))
}

impl Server {
    fn msg_buffer_targets(&self, msg: &IrcMsg) -> Vec<BufferTarget> {
        unimplemented!();
    }

    pub fn add_irc_msg(&mut self, msg: IrcMsg) {
        let now = SteadyTime::now();
        let targets = self.msg_buffer_targets(&msg);
        self.buffer.insert(now, msg);

        self.sub_buffers.insert((QueryBuffer::Any, now));
        match targets.len() {
            0 => {
                self.sub_buffers.insert((QueryBuffer::Server, now));
            },
            _ => {
                self.sub_buffers.extend(targets.into_iter()
                    .map(|target| (QueryBuffer::Target(target), now)));
            }
        }
    }

    pub fn query(&self, query: Query, limit: u8) -> Result<QueryResult, QueryError> {
        use std::collections::Bound::{Excluded, Unbounded};

        let Query { buffer, direction } = query;
        let (dir, res) = match direction {
            QueryDirection::Oldest => {
                let iterator = self.sub_buffers.range(Unbounded, Unbounded)
                    .take(limit as usize);
                match query_helper(iterator, &self.buffer, limit as usize) {
                    Some((k, res)) => (QueryDirection::Later(k), res),
                    None => (QueryDirection::Oldest, Vec::new()),
                }
            },
            QueryDirection::Later(st) => {
                let key = (buffer.clone(), st);
                let iterator = self.sub_buffers.range(Excluded(&key), Unbounded)
                    .take(limit as usize);
                match query_helper(iterator, &self.buffer, limit as usize) {
                    Some((k, res)) => (QueryDirection::Later(k), res),
                    None => (QueryDirection::Later(st), Vec::new()),
                }
            },
            QueryDirection::Earlier(st) => {
                let key = (buffer.clone(), st);
                let iterator = self.sub_buffers.range(Unbounded, Excluded(&key))
                    .rev().take(limit as usize);
                match query_helper(iterator, &self.buffer, limit as usize) {
                    Some((k, res)) => (QueryDirection::Earlier(k), res),
                    None => (QueryDirection::Earlier(st), Vec::new()),
                }
            },
            QueryDirection::Newest => {
                let iterator = self.sub_buffers.range(Unbounded, Unbounded)
                    .rev().take(limit as usize);
                match query_helper(iterator, &self.buffer, limit as usize) {
                    Some((k, res)) => (QueryDirection::Earlier(k), res),
                    None => (QueryDirection::Newest, Vec::new()),
                }
            },
        };
        Ok(QueryResult {
            next: Query {
                buffer: QueryBuffer::Any,
                direction: dir,
            },
            messages: res,
        })
    }
}

#[derive(Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct BufferTarget(Vec<u8>);

impl BufferTarget {
    pub fn new(cm: &OSCaseMapping, target: &[u8]) -> BufferTarget {
        let lower_map = cm.get_lower_map();
        let out = target.iter()
            .map(|&byte| lower_map[byte as usize])
            .collect();
        BufferTarget(out)
    }
}
