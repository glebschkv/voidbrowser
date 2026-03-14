use serde::Serialize;
use uuid::Uuid;

/// Serializable tab info sent to the frontend via IPC.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_loading: bool,
    pub favicon_url: Option<String>,
}

/// Internal tab state.
#[derive(Debug)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_loading: bool,
    pub favicon_url: Option<String>,
}

impl Tab {
    pub fn new(url: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Tab".to_string(),
            url,
            is_loading: false,
            favicon_url: None,
        }
    }

    pub fn to_info(&self) -> TabInfo {
        TabInfo {
            id: self.id.clone(),
            title: self.title.clone(),
            url: self.url.clone(),
            is_loading: self.is_loading,
            favicon_url: self.favicon_url.clone(),
        }
    }
}

/// Manages the collection of open tabs.
pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<String>,
}

#[allow(dead_code)]
impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_id: None,
        }
    }

    pub fn add_tab(&mut self, tab: Tab) -> TabInfo {
        let info = tab.to_info();
        self.tabs.push(tab);
        info
    }

    pub fn remove_tab(&mut self, tab_id: &str) -> Option<Tab> {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == tab_id) {
            let tab = self.tabs.remove(pos);
            // If we removed the active tab, activate an adjacent one
            if self.active_tab_id.as_deref() == Some(tab_id) {
                self.active_tab_id = if !self.tabs.is_empty() {
                    let new_idx = if pos > 0 { pos - 1 } else { 0 };
                    Some(self.tabs[new_idx].id.clone())
                } else {
                    None
                };
            }
            Some(tab)
        } else {
            None
        }
    }

    pub fn get_tab(&self, tab_id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == tab_id)
    }

    pub fn get_tab_mut(&mut self, tab_id: &str) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == tab_id)
    }

    pub fn get_active_tab(&self) -> Option<&Tab> {
        self.active_tab_id
            .as_deref()
            .and_then(|id| self.get_tab(id))
    }

    pub fn set_active(&mut self, tab_id: &str) -> bool {
        if self.tabs.iter().any(|t| t.id == tab_id) {
            self.active_tab_id = Some(tab_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn reorder(&mut self, tab_ids: &[String]) {
        let mut reordered = Vec::with_capacity(self.tabs.len());
        for id in tab_ids {
            if let Some(pos) = self.tabs.iter().position(|t| t.id == *id) {
                reordered.push(self.tabs.remove(pos));
            }
        }
        // Append any tabs not in the provided list (shouldn't happen, but safe)
        reordered.append(&mut self.tabs);
        self.tabs = reordered;
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn all_tab_infos(&self) -> Vec<TabInfo> {
        self.tabs.iter().map(|t| t.to_info()).collect()
    }

    /// Find the index of a tab by id.
    pub fn tab_index(&self, tab_id: &str) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == tab_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_tab() {
        let mut mgr = TabManager::new();
        let tab = Tab::new("https://example.com".to_string());
        let id = tab.id.clone();
        mgr.add_tab(tab);
        assert_eq!(mgr.tab_count(), 1);
        assert!(mgr.get_tab(&id).is_some());
    }

    #[test]
    fn test_remove_tab_activates_adjacent() {
        let mut mgr = TabManager::new();
        let t1 = Tab::new("https://a.com".to_string());
        let t2 = Tab::new("https://b.com".to_string());
        let t3 = Tab::new("https://c.com".to_string());
        let id1 = t1.id.clone();
        let id2 = t2.id.clone();
        let id3 = t3.id.clone();
        mgr.add_tab(t1);
        mgr.add_tab(t2);
        mgr.add_tab(t3);
        mgr.set_active(&id2);

        // Remove active (middle) tab — should activate the one before it
        mgr.remove_tab(&id2);
        assert_eq!(mgr.tab_count(), 2);
        assert_eq!(mgr.active_tab_id.as_deref(), Some(id1.as_str()));

        // Remove first tab — should activate the next (now id3)
        mgr.set_active(&id1);
        mgr.remove_tab(&id1);
        assert_eq!(mgr.active_tab_id.as_deref(), Some(id3.as_str()));
    }

    #[test]
    fn test_remove_last_tab() {
        let mut mgr = TabManager::new();
        let tab = Tab::new("https://example.com".to_string());
        let id = tab.id.clone();
        mgr.add_tab(tab);
        mgr.set_active(&id);
        mgr.remove_tab(&id);
        assert_eq!(mgr.tab_count(), 0);
        assert!(mgr.active_tab_id.is_none());
    }

    #[test]
    fn test_reorder() {
        let mut mgr = TabManager::new();
        let t1 = Tab::new("https://a.com".to_string());
        let t2 = Tab::new("https://b.com".to_string());
        let t3 = Tab::new("https://c.com".to_string());
        let id1 = t1.id.clone();
        let id2 = t2.id.clone();
        let id3 = t3.id.clone();
        mgr.add_tab(t1);
        mgr.add_tab(t2);
        mgr.add_tab(t3);

        mgr.reorder(&[id3.clone(), id1.clone(), id2.clone()]);
        assert_eq!(mgr.tabs[0].id, id3);
        assert_eq!(mgr.tabs[1].id, id1);
        assert_eq!(mgr.tabs[2].id, id2);
    }

    #[test]
    fn test_set_active_invalid() {
        let mut mgr = TabManager::new();
        assert!(!mgr.set_active("nonexistent"));
    }
}
