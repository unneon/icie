
use evscode::TreeData;
use vscode_sys::{TreeItem,TreeItemCollapsibleState,EventEmitter};
use crate::submit::connect_to_workspace_task;
use unijudge::{Backend, Resource, Statement,Problem,
	boxed::{BoxedContest, BoxedTask},ErrorCode
};
use core::time::Duration;
use evscode::R;

//use crate::util::sleep;
use crate::{
	assets, dir, logger, manifest::Manifest, net::{interpret_url, require_task,Session}, open, util::{self, fs, workspace_root,sleep,set_workspace_root}
};
async fn get_prob_list() -> R<Vec<Problem>>{
    let manifest=Manifest::load().await?;
    let url = manifest.req_task_url()?;
    let (url, backend) = interpret_url(url)?;
    let url = require_task::<BoxedContest, BoxedTask>(url)?;
    let Resource::Task(task) = url.resource;
    let sess = Session::connect(&url.domain, backend).await?;
    
    let prob_list= sess.run(|backend, sess| backend.problems_list(sess, &task)).await?;
    Ok(prob_list)
}
async fn get_child(element:Option<TreeItem>) -> Vec<TreeItem> {
    let mut vec = Vec::new();
    match element{
        Some(elem)=>{
            //sleep(Duration::from_secs(1)).await;
            vec
        },
        _ =>{
             if let Ok(prob_list) = get_prob_list().await{
                for it in prob_list.into_iter(){
                    vec.push(TreeItem{
                        label:format!("{:<25}-{:>5}",it.name,it.total_submissions),
                        collapse:0,
                        icon: if(it.status ==0) 
                            {evscode::get_path("assets/accept.png")}
                        else if(it.status ==1)
                            {evscode::get_path("assets/reject.png")} 
                        else 
                            {evscode::get_path("assets/minus.png")} 
                    });
                }
             }     else {
                vec.push(TreeItem{
                    label:"Not Available".to_string(),
                    collapse:0,
                    icon:  evscode::get_path("assets/minus.png")
                });
             }
            //let (sess, task) = connect_to_workspace_task().await.unwrap();
            //let problist = sess.run(|backend, sess| backend.problems_list(sess, &task)).await.unwrap();
            //sleep(Duration::from_secs(1)).await;
            
            vec
        }

    }
}
async fn isvisible() -> bool {
    if let Ok(manifest) = Manifest::load().await {
        true
    }else {
        false
    }
}



thread_local!{
    static SUBMISSIONS_VIEW_EVENT:EventEmitter = EventEmitter::new();
}

async fn refresh() -> R<()> {
    SUBMISSIONS_VIEW_EVENT.with(|x| x.fire());
    Ok(())
}

#[evscode::contribview(name = "Submissions", addto = "explorer")]
static treedataprovider:TreeData = TreeData{
    getTreeItem:&|element| {
        return element;
    },
    getChildren:&|element| Box::pin(get_child(element)),
    refreshevent:SUBMISSIONS_VIEW_EVENT,
    refresh:&|| Box::pin(refresh()),
    /*isvisible:&|| Box::pin(isvisible()),*/
};
/*impl TreeData{
    // refreshevet:;
    refresh:&||{
        
    },
}*/