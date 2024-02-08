// 1. formula 3041 event state
// k : 3041_event-id_state
// v:  event_state


use std::collections::HashMap;
use std::str::Utf8Error;
use std::string::ToString;
use nostr_kv::lmdb::Db;

use nostr_kv::{Error, lmdb::{Db as Lmdb, Iter as LmdbIter, *}, scanner::{Group, GroupItem, MatchResult, Scanner}};
use proto::zchronod::Event;

use serde::{Serialize, Deserialize};
use log::info;

pub struct ZchronodDb {
    inner: Db,
    state: Tree,
}

#[derive(Serialize, Deserialize)]
pub struct OptionState {
    // map: HashMap<String, i32>,
    option_vec: Vec<(String, i32)>,
    // option_name : vote_num
    event: Event,
}

type Result<T, E = Error> = core::result::Result<T, E>;

const TREE_NAME: &str = "3041";

impl ZchronodDb {
    // kind_301_poll is to init kv
    // kind_309_vote is to update value
    // k : 3041_event-id_state
    // v:  event_state
    // event_state is  map[option_name]vote_num, event
    pub fn init() -> Result<Self> {
        let lmdb = Db::open("./db")?;
        let state = lmdb.open_tree(Some(TREE_NAME), 0)?;
        Ok(ZchronodDb {
            inner: lmdb,
            state,
        })
    }

    pub fn writer(&self) -> Result<Writer> {
        Ok(self.inner.writer()?)
    }

    pub fn reader(&self) -> Result<Reader> {
        Ok(self.inner.reader()?)
    }


    // let my_path = "./my_file_sglk";
    // let db = Db::open(my_path)?;
    // // let _t = db.open_tree(None, 0)?;
    // let t1 = db.open_tree(Some("t2"), 0)?;
    //
    // // let mut writer = db.writer()?;
    // // writer.put(&t1,b"k1", b"v1")?;
    // let reader = db.reader()?;
    // let _v2 = reader.get(&t1,"k1")?.unwrap();
    // println!("{:?}",std::str::from_utf8(_v2));
    // init kv

    // all poll_id in db is poll_id=event id, vec<string>

    //   let event_id_str = String::from_utf8_lossy(&event_id);

    // fn get_vote_null_option(e: Event) -> HashMap<String, i32> {
    //
    // }
    pub fn poll_write(&self, key: String, e: Event) -> Result<(), Error> {
        println!("poll write key is {:?}", key.clone());
        let reader = self.inner.reader()?;
        if reader.get(&self.state, key.clone())?.is_none() {
            let mut writer = self.inner.writer()?;
            // convert option_state to json, and write as bytes
            if e.tags.len() != 1 {
                println!("tag len != 1, should be panic");
                panic!()
            }
            let poll_tag = e.tags.get(0).unwrap().clone().values;
            // option start with index 5
            // let mut option_hmap: HashMap<String, i32> = HashMap::new();
            let mut option_vec: Vec<(String, i32)> = vec![];
            for i in 5..=poll_tag.len() - 1 {
                //option_hmap.insert(poll_tag.get(i).unwrap().to_string(), 0);
                option_vec.push((poll_tag.get(i).unwrap().to_string(), 0));
                println!("insert index {} , which is {}", i, poll_tag.get(i).unwrap().to_string());
            }
            let o_s = OptionState {
                // map: option_hmap,    // to generate option with 0
                option_vec,
                event: e.clone(),

            };
            let option_state = serde_json::to_string(&o_s).unwrap();
            writer.put(&self.state, key.clone(), option_state);
            match reader.get(&self.state, "poll_id".to_string())? {
                Some(t) => {
                    let mut poll_id_list: Vec<Vec<u8>> = serde_json::from_str(std::str::from_utf8(t).unwrap()).unwrap();
                    poll_id_list.push(e.id.clone());
                    writer.put(&self.state, "poll_id".to_string(), serde_json::to_string(&poll_id_list).unwrap());
                }
                None => { writer.put(&self.state, "poll_id".to_string(), e.id.clone()); }
            }
            writer.commit()?;
        }
        Ok(())
    }

    pub fn vote_write(&self, e: Event) -> Result<(), Error> {
        // construct key
        let mut vote_tag = e.tags.clone();
        let mut event_id = "".to_string();
        let mut option_vote: Vec<String> = vec![];
        let event_symbol = "e".to_string();

        // should be once in item
        for item in &mut vote_tag {
            if item.values.get(0).unwrap().to_string() == event_symbol {
                event_id = item.values.get(1).unwrap().to_string();
            }
            if item.values.get(0).unwrap().to_string() == "poll_r".to_string() {
                for i in 1..=item.values.len() - 1 {
                    option_vote.push(item.values.get(i).unwrap().to_string());
                    println!("insert poll_r index {} , which is {}", i, item.values.get(i).unwrap().to_string());
                }
            }
        }

        let key = format!("3041_{}_state", event_id);
        println!("vote write key is {:?}", key.clone());
        // read state, update, write
        let reader = self.inner.reader()?;

        let state = std::str::from_utf8(reader.get(&self.state, key.to_string())?.unwrap()).unwrap();
        let mut op_read_state: OptionState = serde_json::from_str(state).unwrap();

        // update
        for vote in &option_vote {
            let vote_index: usize = vote.parse().unwrap();
            if let Some(mut tuple) = op_read_state.option_vec.get_mut(vote_index) {
                tuple.1 += 1;
                println!("{:?}", tuple);
            }
        }

        // write
        let mut writer = self.inner.writer()?;
        let wirte_json_string = serde_json::to_string(&op_read_state).unwrap();
        writer.put(&self.state, key.to_string(), wirte_json_string);
        writer.commit()?;

        Ok(())
    }

    pub fn query_poll_event_state(&self, event_id: String) -> Result<Vec<String>, Error> {
        // construct key
        let key = format!("3041_{}_state", event_id);
        println!("vote write key is {:?}", key.clone());
        let reader = self.inner.reader()?;

        let state = std::str::from_utf8(reader.get(&self.state, key.to_string())?.unwrap()).unwrap();

        let  op_state: OptionState = serde_json::from_str(state).unwrap();
        let mut result:Vec<String> =vec![];
        for element in op_state.option_vec{
            result.push(element.0);
        }
        Ok(result)
    }
    pub fn query_all_event_id(&self) -> Result<(Vec<Vec<u8>>), Error> {
        let reader = self.inner.reader()?;
        match reader.get(&self.state, "poll_id".to_string())? {
            Some(t) => {
                let poll_id_list: Vec<Vec<u8>> = serde_json::from_str(std::str::from_utf8(t).unwrap()).unwrap();
                Ok::<Vec<Vec<u8>>, Error>(poll_id_list)
            }
            None => {
                println!("find none in query_all_event_id");
                info!("find none in query_all_event_id");
                Ok(vec![vec![]])
            }
        }.expect("err in query_all_event_id");

        Ok(vec![vec![]])
    }

    fn write_3041_db(&self, key: &str, option_state: HashMap<String, i32>) -> Result<(), Error> {
        let reader = self.inner.reader()?;
        let mut op_state = "".to_string();
        match reader.get(&self.state, key.to_string())? {
            None => {
                return Err(Error::Message("failed to get state in db".to_string()));
            }
            Some(t) => {
                let state_bytes = reader.get(&self.state, key.to_string());
                match reader.get(&self.state, key.to_string()) {
                    Ok(s) => {
                        match std::str::from_utf8(s.unwrap()) {
                            Ok(i) => { op_state = i.to_string() }
                            Err(_) => {
                                return Err(Error::Message("failed to transfer to string".to_string()));
                            }
                        }
                    }
                    Err(_) => { return Err(Error::Message("failed to get state in db".to_string())); }
                }
                //  op_state = std::str::from_utf8(state_bytes);
            }
        }


        Ok(())
    }

    fn query_by_event_id() {}
}