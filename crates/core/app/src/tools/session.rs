//! Opening, closing and listing files.


use crate::data::{data, Data};
use crate::{Command, Query};
use crate::{Output, ToolError, Toolkit};

/// A file opened and kept open, its unsaved edits replayed or discarded.
///
/// The replay runs inside this command, as it always has, so a recovery is
/// journaled as an `open_file` of its own.
pub struct OpenFile {
    pub path: std::path::PathBuf,
    pub recover: bool,
}

impl Command for OpenFile {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let (path, recover) = (self.path.as_path(), self.recover);
        let open_before: Vec<crate::Handle> = kit.sessions().map(|s| s.handle.clone()).collect();
        let handle = kit.open(path)?.handle.clone();
        if !open_before.contains(&handle) {
            kit.session_mut(handle.as_str())?.recovery = Some(recover);
        }
        let session = kit.session(handle.as_str())?;
        let mut out = super::read::summary(session);
        out["handle"] = data!(session.handle);
        out["open_seconds"] = data!((session.open_seconds * 100.0).round() / 100.0);
        Ok(Output::Data(out))
    }
}


/// A file closed, its unsaved edits discarded.
pub struct CloseFile {
    pub handle: String,
}

impl Command for CloseFile {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.close(&self.handle)?;
        kit.recovery().discard(kit, &session);
        Ok(Output::Data(data!({
            "closed": true,
            "file": session.file_name(),
            "unsaved_edits": session.edits,
        })))
    }
}


/// The files open.
pub struct ListOpen;

impl Query for ListOpen {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let files: Vec<Data> = kit
            .sessions()
            .map(|s| {
                data!({
                    "handle": s.handle,
                    "path": s.path.display().to_string(),
                    "nodes": s.graph.nodes.len(),
                    "edits": s.edits,
                    "opened_at": s.opened_at,
                    "open_seconds": (s.open_seconds * 100.0).round() / 100.0,
                    "opened_delta_mb": s.opened_delta_mb,
                })
            })
            .collect();
        Ok(Output::Data(data!({ "files": files, "process_resident_mb": crate::resident_mb() })))
    }
}

