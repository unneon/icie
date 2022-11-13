//! TreeDataProvider functions for TreeView

use vscode_sys::{TreeItem, TreeItemCollapsibleState};
//use serde::{Serialize, Deserialize};
use wasm_bindgen::{closure::Closure, JsValue};
use std::ops::Deref;
use once_cell::sync::Lazy;
use crate::{BoxFuture,R,State};
use js_sys;
use std::future::Future;
use core::time::Duration;
use vscode_sys::{Event,EventEmitter,Thenable};
use core::marker::PhantomData;
use std::thread::LocalKey;
/// TreeData functions for TreeView

//pub type LazyFutureChildren = dyn (Fn(Option<TreeItem>) -> Future<Output=Vec<TreeItem>>) +Sync;
//pub type LazyFutureChildren =   Box<(Fn(Option<TreeItem>) -> BoxFuture<'static,Option<Vec<TreeItem>>>) +Sync>;
pub type LazyFutureChildren =   (Fn(Option<TreeItem>) -> BoxFuture<'static,Vec<TreeItem>>) +Sync;
//pub type LazyFutureChildren =   Fn(Option<TreeItem>) ->  Future<Output=Option<Vec<TreeItem>>> +Sync;

/// TreeData functions for TreeView
pub type LazyFutureItem = dyn (Fn(TreeItem)->TreeItem)+Sync;
/// TreeData functions for TreeView
pub type LazyRefresh = dyn (Fn()->BoxFuture<'static,R<()>>)+Sync;
/// TreeData functions for TreeView
pub type Lazyisvisible =   (Fn() -> BoxFuture<'static,bool>) +Sync;
/// TreeData functions for TreeView
pub type Refreshevent = (Fn() ->LocalKey<EventEmitter>) +Sync;


//use serde_closure::desugar;
/// TreeData functions for TreeView
//#[derive(Serialize)]
//#[wasm_bindgen::prelude::wasm_bindgen]
pub struct TreeData{
    /// TreeData functions for TreeView
    //pub getChildren:&'static LazyFutureChildren,
    /// TreeData functions for TreeView
    pub getTreeItem:&'static LazyFutureItem,
    /// TreeData functions for TreeView
    pub getChildren:&'static LazyFutureChildren,
    
    ///TreeData functions for TreeView
   pub refresh:&'static LazyRefresh,
     ///   TreeData functions for TreeView
    pub refreshevent:LocalKey<EventEmitter>,
  // pub isvisible:&'static Lazyisvisible,
 
}

/*impl TreeData{
    pub fn set_fire_event(&mut self)->EventEmitter{
        let refreshevent:EventEmitter= EventEmitter::new();
        let clos=Closure::new(|| {
            refreshevent.fire();
        });
        self.refresh=&||{
            //(clos)();
        };
        refreshevent
    }
}*/
//#[wasm_bindgen::prelude::wasm_bindgen]
/*pub struct TreeDataProvider{
    pub getChildren:Closure<dyn FnMut()>,
  //  pub getTreeItem:JsValue,
}*/
use serde::ser::{Serialize,Serializer,SerializeStruct};
extern crate wasm_bindgen;
use lazy_static::lazy_static;
use wasm_bindgen::JsCast;

#[wasm_bindgen::prelude::wasm_bindgen]
/// convert to jsvalue
pub struct ClosureHandle(Closure<FnMut(JsValue)->Future<Output=JsValue>>);
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::Promise;
use std::sync::Arc;
/// convert to jsvalue
/*const Treedata_event: evscode::State<String> =
	evscode::State::new("icie.newsletter.lastAcknowledgedVersion", evscode::state::Scope::Global);*/
    /// convert to jsvalue
    pub fn to_JsValue(treedataobj:&'static TreeData)->JsValue{
        //let close=Closure::wrap(Box::new(self.getTreeItem));
        //Closure::once_into_js(self.getChildren);
        /*Closure::new(async move |elem| ->Option<Vec<TreeItem>>{
                
        });*/
        /*let f1= Closure::new(
             move|elem:Option<TreeItem>|->Vec<TreeItem> {
                //let mut ret= Vec::new();
            //async move{
             let mut ret=(self.getChildren)(elem).await;
             
            //};
            ret
            }
        );*/
        //let shared=Box::leak(Box::new(self.deref()));
        
        //Closure::new::<Closure<dyn FnMut(TreeItem)->Promise>>( |elem| ->Promise{
            //<Closure<dyn FnMut(Option<TreeItem>)->R<(JsValue)>>
        /*Closure::new( |elem| ->R<JsValue>{
        //    wasm_bindgen_futures::future_to_promise(
                async move {
                
                let ret=(self.clone().getChildren)(Some(elem)).await;
                let objar = js_sys::Array::new();
                for it in ret.into_iter(){
                    objar.push(&it);
                }
                Ok(objar.into())
            }
        //);
        });*/
        /*let f1= Closure::<FnMut(JsValue)->Future<Output=JsValue>>::new(Box::new(async move |elem:JsValue|{
            let inp_data:TreeItem=elem.into_serde().unwrap();
            let objar = js_sys::Array::new();
            //wasm_bindgen_futures::future_to_promise(
                //async move {
                let ret=(treedataobj.getChildren)(Some(inp_data)).await;
                //let objar = js_sys::Array::new();
                for it in ret.into_iter(){
                    objar.push(&JsValue::from_serde(&it).unwrap());
                }
                //Ok(objar.into())
           // }
           objar.into()  
                
        }) as Box<dyn FnMut(JsValue)->Future<Output=JsValue>>);*/
        
        let f1= Closure::<FnMut(JsValue)->Promise>::wrap(Box::new(move |elem:JsValue|{
            let inp_data:Option<TreeItem>;
            if(elem.is_null() || elem.is_undefined()){
                inp_data=None;
            }else{
                inp_data=Some(elem.into_serde().unwrap());
            }
            let objar = js_sys::Array::new();
            wasm_bindgen_futures::future_to_promise(async move {
                let ret=(treedataobj.getChildren)(inp_data).await;
                //let objar = js_sys::Array::new();
                for it in ret.into_iter(){
                    objar.push(&JsValue::from_serde(&it).unwrap());
                }
                Ok(objar.into())
            })
                
                
        }) as Box<dyn FnMut(JsValue)->Promise>);
        
        let f2= Closure::<FnMut(JsValue)->JsValue>::wrap(Box::new(move |elem:JsValue|{
            let inp_data:TreeItem=elem.into_serde().unwrap();
            
            let objar = js_sys::Array::new();
                let ret=(treedataobj.getTreeItem)(inp_data);
                //let objar = js_sys::Array::new();
                JsValue::from_serde(&ret).unwrap()
            }));
                
        /*let f3= Closure::<FnMut()->Promise>::wrap(Box::new(move ||{
            wasm_bindgen_futures::future_to_promise(async move {
                let ret=(treedataobj.isvisible)().await;
                Ok(ret.into())
            })
                
                
        }) as Box<dyn FnMut()->Promise>);*/

        let obj = js_sys::Object::new();
        //js_sys::Reflect::set(&obj, &"getTreeItem".into(), &Closure::once_into_js(self.clone().getTreeItem));
        js_sys::Reflect::set(&obj, &"getChildren".into(), &f1.as_ref().unchecked_ref());
        js_sys::Reflect::set(&obj, &"getTreeItem".into(), &f2.as_ref().unchecked_ref());
        //js_sys::Reflect::set(&obj, &"isvisible".into(), &f3.as_ref().unchecked_ref());
        f1.forget();
        f2.forget();
        //f3.forget();
        //let refreshevent:EventEmitter= EventEmitter::new();
        treedataobj.refreshevent.with(|event|{
            js_sys::Reflect::set(&obj, &"onDidChangeTreeData".into(), &event.get_event());
        });
        
        /*treedataobj.refresh=& || {
            //refreshevent.fire();
        };*/
        //let f4= Closure::<FnMut()>::wrap(Box::new(move ||{
        //        refreshevent.fire();
        //    }));
       // js_sys::Reflect::set(&obj, &"refresh".into(), &f4.as_ref().unchecked_ref());     


        /*js_sys::Reflect::set(&obj, &"getChildren".into(), &Closure::once_into_js(async{
            Ok((self.getChildren)().await)
        });*/
        //js_sys::Reflect::set(&obj, &"getChildren".into(), &wasm_bindgen_futures::future_to_promise(&self.getChildren));
        obj.into()
    }


/*impl TreeData{
    fn to_JsValue(&self)->JsValue{
        let to_js:TreeDataProvider=TreeDataProvider{
            getChildren:
            getTreeItem:Closure::once_into_js(self.getTreeItem),
        }
        JsValue::from_serde(&to_js).unwrap()
    }
}
impl TreeData{
    fn to_JsValue(&self)->JsValue{
        let to_js:TreeDataProvider=TreeDataProvider{
            getChildren:Closure::once_into_js(self.getTreeItem),
            getTreeItem:Closure::once_into_js(self.getTreeItem),
        }
        JsValue::from_serde(&to_js).unwrap()
    }
}
impl Serialize for TreeDataProvider {
    fn serialize<S>(&self, serializer: S) -> Result<<S>::Ok, <S>::Error> where S: Serializer
    {
        /*let obj=self;
        let f1=Fn!( ||->Option<Vec<TreeItem>> {
            Some(vec![TreeItem::new("First",TreeItemCollapsibleState::None)])
        });
        let f2=Fn!( |element|->TreeItem {(element)});*/
        
        let mut state = serializer.serialize_struct("TreeDataProvider", 2)?;
        state.serialize_field("getChildren", &self.getChildren)?;
        //state.serialize_field("getTreeItem", &f2)?;
        //state.serialize_field("getTreeItem",&Closure::once_into_js(self.getTreeItem))?;
        state.end()
    }
}*/
/*
impl Serialize for TreeData {
    fn serialize<S>(&self, serializer: S) -> Result<<S>::Ok, <S>::Error> where S: Serializer
    {
        //use serde_closure::{traits::Fn, Fn};
        let obj=self;
        /*let closure = Fn!(|element| {
			(obj.getChildren)(element);
		});*/
        //let f2=Fn!( |element|->TreeItem {(obj.getTreeItem)(element)});
        let closure1 = 
            Fn!(|element:TreeItem| {
                //(obj.getChildren)(element);
                vec![TreeItem::new("First",TreeItemCollapsibleState::None)]

            });
        let closure2 =
            Fn!(|element:TreeItem| {
                element
            });
        let mut state = serializer.serialize_struct("TreeDataProvider", 2)?;
        state.serialize_field("getChildren", &closure1)?;
        state.serialize_field("getTreeItem", &closure2)?;
        //state.serialize_field("getTreeItem", &self.getTreeItem)?;
        //state.serialize_field("getChildren", &self.getChildren)?;
        //state.serialize_field("getTreeItem", &f2)?;
        //state.serialize_field("getTreeItem",&Closure::once_into_js(self.getTreeItem))?;
        state.end()
    }
}*/
