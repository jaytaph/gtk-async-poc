use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

// We use this class as a immutable singleton. This is bad.
pub struct TabManager {
    tab_pages: RwLock<HashMap<Uuid, u32>>,
    tabs: RwLock<HashMap<Uuid, TabInfo>>
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TabInfo {
    pub tab_id: Uuid,
    pub favicon: Vec<u8>,
    pub url: Option<String>,
    pub title: Option<String>,
}

impl TabInfo {
    pub(crate) fn new(tab_id: Uuid, url: String) -> TabInfo {
        TabInfo {
            tab_id,
            favicon: Vec::new(),
            url: Some(url.clone()),
            title: Some(url.clone()),
        }
    }
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tab_pages: RwLock::new(HashMap::new()),
            tabs: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_tab_info(&self, tab_id: Uuid) -> Option<TabInfo> {
        match self.tabs.read().unwrap().get(&tab_id) {
            Some(tab_info) => Some(tab_info.clone()),
            None => None,
        }
    }

    pub fn add(&self, tab_id: Uuid, page_num: u32, tab_info: TabInfo) {
        self.tab_pages.write().unwrap().insert(tab_id, page_num);
        self.tabs.write().unwrap().insert(tab_id, tab_info);
    }

    pub fn get_by_tab(&self, tab_id: Uuid) -> Option<u32> {
        self.tab_pages.read().unwrap().get(&tab_id).copied()
    }

    #[allow(dead_code)]
    pub fn get_by_page(&self, page_num: u32) -> Option<Uuid> {
        self.tab_pages.read().unwrap().iter().find_map(|(tab_id, num)| {
            if *num == page_num {
                Some(*tab_id)
            } else {
                None
            }
        })
    }
}