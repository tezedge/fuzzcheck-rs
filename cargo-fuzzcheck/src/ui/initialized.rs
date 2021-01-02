use std::{collections::HashMap, ffi::OsString, rc::Rc};

use fuzzcheck_common::arg::ResolvedCommandLineArguments;
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::project::Root;

use super::{
    framework::{Either, Focusable, HorizontalMove, InnerFocusable, ParentView, Theme, VerticalMove, ViewState},
    horizontal_list_view::{self, HorizontalListView},
    run_fuzz::{self, RunFuzzView},
};

pub struct InitializedView {
    pub root: Rc<Root>,
    focus: Focus,
    fuzz_target_list: HorizontalListView,
    run_fuzz_views: Vec<RunFuzzView>,
}

impl InitializedView {
    pub fn new(root: Rc<Root>) -> Self {
        let fuzz_target_list = HorizontalListView::new(
            "Fuzz Targets",
            fuzz_targets_from_root(&root)
                .keys()
                .map(|k| k.to_str().unwrap().to_string()),
        );

        let run_fuzz_views = fuzz_target_list
            .items
            .iter()
            .map(|fuzz_target| RunFuzzView::new(root.clone(), fuzz_target.clone()))
            .collect();
        let focus = Focus::Sidebar;

        let mut res = Self {
            root: root,
            focus,
            fuzz_target_list,
            run_fuzz_views,
        };
        res.update_focus(res.focus);
        res
    }
}

impl InitializedView {
    fn current_run_fuzz_view(&self) -> Option<&RunFuzzView> {
        if let Some(selected) = self.fuzz_target_list.state.selected() {
            self.run_fuzz_views.get(selected)
        } else {
            None
        }
    }
    fn current_run_fuzz_view_as_mut(&mut self) -> Option<&mut RunFuzzView> {
        if let Some(selected) = self.fuzz_target_list.state.selected() {
            self.run_fuzz_views.get_mut(selected)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub enum Focus {
    Sidebar,
    Main,
}

pub enum Update {
    Sidebar(HorizontalMove),
    RunFuzz(run_fuzz::Update),
    SwitchFocus(Focus),
    SelectTarget(usize),
}

pub enum OutMessage {
    Run(ResolvedCommandLineArguments),
}

impl InnerFocusable for InitializedView {
    type Focus = self::Focus;

    fn focus(&mut self) -> &mut Self::Focus {
        &mut self.focus
    }

    fn view_in_focus(&mut self) -> Option<&mut dyn Focusable> {
        match self.focus {
            Focus::Sidebar => Some(&mut self.fuzz_target_list),
            Focus::Main => {
                if self.fuzz_target_list.state.selected().is_none() {
                    Some(&mut self.fuzz_target_list)
                } else {
                    Some(self.current_run_fuzz_view_as_mut().unwrap())
                }
            }
        }
    }
}

impl ViewState for InitializedView {
    type Update = self::Update;
    type InMessage = Key;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match self.focus {
            Focus::Sidebar => Self::handle_child_in_message(&self.fuzz_target_list, message).or_else(|| {
                if let Some(VerticalMove::Down) = VerticalMove::from(&message) {
                    Some(Update::SwitchFocus(Focus::Main))
                } else if matches!(message, Key::Char('\n') | Key::Esc) {
                    Some(Update::SwitchFocus(Focus::Main))
                } else {
                    None
                }
            }),
            Focus::Main => {
                if let Some(run_fuzz) = self.current_run_fuzz_view() {
                    Self::handle_child_in_message(run_fuzz, message).or_else(|| {
                        if matches!(message, Key::Esc) {
                            Some(Update::SwitchFocus(Focus::Sidebar))
                        } else if let Some(VerticalMove::Up) = VerticalMove::from(&message) {
                            Some(Update::SwitchFocus(Focus::Sidebar))
                        } else {
                            None
                        }
                    })
                } else {
                    Some(Update::SwitchFocus(Focus::Sidebar))
                }
            }
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        match u {
            Update::Sidebar(u) => self
                .fuzz_target_list
                .update(u)
                .and_then(|out| <Self as ParentView<HorizontalListView>>::handle_child_out_message(self, out)),
            Update::RunFuzz(u) => self
                .current_run_fuzz_view_as_mut()
                .and_then(|run_fuzz| run_fuzz.update(u))
                .and_then(|out| <Self as ParentView<RunFuzzView>>::handle_child_out_message(self, out)),
            Update::SwitchFocus(f) => {
                self.update_focus(f);
                None
            }
            Update::SelectTarget(_target) => {
                //self.run_fuzz = Some(RunFuzzView::new(self.root.clone(), target));
                None
            }
        }
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);

        self.fuzz_target_list.draw(frame, theme, chunks[0]);

        if let Some(run_fuzz) = self.current_run_fuzz_view() {
            run_fuzz.draw(frame, theme, chunks[1])
        }
    }
}

impl ParentView<HorizontalListView> for InitializedView {
    fn convert_child_update(update: <HorizontalListView as ViewState>::Update) -> Self::Update {
        Self::Update::Sidebar(update)
    }

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<HorizontalMove> {
        HorizontalMove::from(&message)
    }

    fn convert_child_out_message(
        &self,
        message: horizontal_list_view::OutMessage,
    ) -> super::framework::Either<Update, OutMessage> {
        match message {
            horizontal_list_view::OutMessage::Select(target) => Either::Left(Update::SelectTarget(target)),
        }
    }
}

impl ParentView<RunFuzzView> for InitializedView {
    fn convert_child_update(update: run_fuzz::Update) -> Self::Update {
        Self::Update::RunFuzz(update)
    }

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<<RunFuzzView as ViewState>::InMessage> {
        Some(message)
    }

    fn convert_child_out_message(&self, message: run_fuzz::OutMessage) -> super::framework::Either<Update, OutMessage> {
        match message {
            run_fuzz::OutMessage::Run(args) => Either::Right(OutMessage::Run(args)),
        }
    }
}

fn fuzz_targets_from_root(root: &Root) -> &HashMap<OsString, Vec<u8>> {
    &root.fuzz.non_instrumented.fuzz_targets.targets
}
