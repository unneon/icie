//! TreeDataProvider functions for TreeView

use vscode_sys::{TreeItem, TreeItemCollapsibleState};
//use serde::{Serialize, Deserialize};
//use wasm_bindgen::{closure::Closure, JsValue};
use std::ops::Deref;
use once_cell::sync::Lazy;
/// TreeData functions for TreeView
pub type LazyFutureChildren = dyn (Fn(Option<TreeItem>) -> Option<Vec<TreeItem>>)+Sync ;
/// TreeData functions for TreeView
pub type LazyFutureItem = dyn (Fn(TreeItem)->TreeItem)+Sync;

use serde_closure::{Fn};
//use serde_closure::desugar;
/// TreeData functions for TreeView
//#[derive(Serialize)]
pub struct TreeData{
    /// TreeData functions for TreeView
    pub getChildren:&'static LazyFutureChildren,
    /// TreeData functions for TreeView
    pub getTreeItem:&'static LazyFutureItem,
}

use serde::ser::{Serialize,Serializer,SerializeStruct};

impl Serialize for TreeData {
    fn serialize<S>(&self, serializer: S) -> Result<<S>::Ok, <S>::Error> where S: Serializer
    {
        let obj=self;
        let f1=Fn!(move |element| (obj.getChildren)(element));
        let f2=Fn!(move |element| (obj.getTreeItem)(element));
        let mut state = serializer.serialize_struct("TreeData", 2)?;
        state.serialize_field("getChildren", &f1)?;
        state.serialize_field("getTreeItem", &f2)?;
        //state.serialize_field("getTreeItem",&Closure::once_into_js(self.getTreeItem.deref()))?;
        state.end()
    }
}
