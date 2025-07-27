use r3bl_tui::{
    col, new_style, render_ops, render_tui_styled_texts_into, row, send_signal, throws_with_return,
    tui_color, tui_styled_text, tui_styled_texts, BoxedSafeComponent, Component, EventPropagation,
    FlexBox, FlexBoxId, GlobalData, HasFocus, InputEvent, Key, KeyPress, RenderOp, RenderPipeline,
    SpecialKey, SurfaceBounds, TerminalWindowMainThreadSignal,
};

use super::{AppSignal, State};

pub struct DeleteConfirmDialog {
    pub id: FlexBoxId,
}

impl DeleteConfirmDialog {
    pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
        Box::new(Self { id })
    }
}

impl Component<State, AppSignal> for DeleteConfirmDialog {
    fn reset(&mut self) {
        // Nothing to reset
    }

    fn get_id(&self) -> FlexBoxId {
        self.id
    }

    fn handle_event(
        &mut self,
        global_data: &mut GlobalData<State, AppSignal>,
        input_event: InputEvent,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            let state = &global_data.state;
            let mut event_consumed = false;

            if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                match key {
                    Key::Character('y') | Key::Character('Y') => {
                        event_consumed = true;
                        let index = state.selected_index;
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::DeleteTask(
                                index
                            ))
                        );
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::ShowMessage(
                                "Task deleted successfully".to_string()
                            ))
                        );
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::CloseDialog)
                        );
                    }
                    Key::Character('n') | Key::Character('N') => {
                        event_consumed = true;
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::CloseDialog)
                        );
                    }
                    Key::SpecialKey(SpecialKey::Esc) => {
                        event_consumed = true;
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::CloseDialog)
                        );
                    }
                    _ => {}
                }
            }

            if event_consumed {
                EventPropagation::ConsumedRender
            } else {
                EventPropagation::Consumed
            }
        })
    }

    fn render(
        &mut self,
        global_data: &mut GlobalData<State, AppSignal>,
        current_box: FlexBox,
        _surface_bounds: SurfaceBounds,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            // Only render if dialog is shown
            if !global_data.state.show_delete_dialog {
                return Ok(RenderPipeline::default());
            }

            let mut render_pipeline = RenderPipeline::default();
            let mut render_ops = render_ops!();
            let state = &global_data.state;

            // Use box bounds for dialog dimensions
            let box_bounds_size = current_box.style_adjusted_bounds_size;
            let box_origin = current_box.style_adjusted_origin_pos;

            // Fixed dialog size
            let dialog_width = 50.min(box_bounds_size.col_width.as_usize());
            let dialog_height = 8.min(box_bounds_size.row_height.as_usize());

            // Center the dialog within the box
            let x = (box_bounds_size
                .col_width
                .as_usize()
                .saturating_sub(dialog_width))
                / 2;
            let y = (box_bounds_size
                .row_height
                .as_usize()
                .saturating_sub(dialog_height))
                / 2;

            // Draw dialog background
            for row_offset in 0..dialog_height {
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x) + row(y + row_offset),
                ));
                render_ops.push(RenderOp::SetBgColor(tui_color!(hex "#2A1E1E")));
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    " ".repeat(dialog_width).into(),
                    None,
                ));
            }

            // Draw border with red color
            // Top border
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x) + row(y),
            ));
            render_ops.push(RenderOp::SetFgColor(tui_color!(hex "#FF0000")));
            let top_border = format!("╔{}╗", "═".repeat(dialog_width - 2));
            render_ops.push(RenderOp::PaintTextWithAttributes(top_border.into(), None));

            // Side borders
            for i in 1..dialog_height - 1 {
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x) + row(y + i),
                ));
                render_ops.push(RenderOp::PaintTextWithAttributes("║".into(), None));
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x + dialog_width - 1) + row(y + i),
                ));
                render_ops.push(RenderOp::PaintTextWithAttributes("║".into(), None));
            }

            // Bottom border
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x) + row(y + dialog_height - 1),
            ));
            let bottom_border = format!("╚{}╝", "═".repeat(dialog_width - 2));
            render_ops.push(RenderOp::PaintTextWithAttributes(
                bottom_border.into(),
                None,
            ));

            // Dialog title
            let title_text = tui_styled_texts! {
                tui_styled_text!{
                    @style: new_style!(bold color_fg: {tui_color!(hex "#FF0000")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: "Confirm Delete"
                },
            };
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x + 2) + row(y + 1),
            ));
            render_tui_styled_texts_into(&title_text, &mut render_ops);

            // Confirmation message
            if let Some(task) = state.tasks.get(state.selected_index) {
                let message = format!("Delete task '{}'?", task.slug);
                let message_text = tui_styled_texts! {
                    tui_styled_text!{
                        @style: new_style!(color_fg: {tui_color!(hex "#FFFFFF")} color_bg: {tui_color!(hex "#2A1E1E")}),
                        @text: message
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x + 2) + row(y + 3),
                ));
                render_tui_styled_texts_into(&message_text, &mut render_ops);
            }

            // Action buttons
            let buttons_text = tui_styled_texts! {
                tui_styled_text!{
                    @style: new_style!(color_fg: {tui_color!(hex "#AAAAAA")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: "Press "
                },
                tui_styled_text!{
                    @style: new_style!(bold color_fg: {tui_color!(hex "#FF0000")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: "Y"
                },
                tui_styled_text!{
                    @style: new_style!(color_fg: {tui_color!(hex "#AAAAAA")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: " to confirm, "
                },
                tui_styled_text!{
                    @style: new_style!(bold color_fg: {tui_color!(hex "#00FF00")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: "N"
                },
                tui_styled_text!{
                    @style: new_style!(color_fg: {tui_color!(hex "#AAAAAA")} color_bg: {tui_color!(hex "#2A1E1E")}),
                    @text: " to cancel"
                },
            };

            // Center the buttons text
            let buttons_width = buttons_text.display_width();
            let buttons_x = x + (dialog_width - buttons_width.as_usize()) / 2;
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(buttons_x) + row(y + 5),
            ));
            render_tui_styled_texts_into(&buttons_text, &mut render_ops);

            render_ops.push(RenderOp::ResetColor);
            render_pipeline.push(ZOrder::Glass, render_ops);
            render_pipeline
        })
    }
}

use r3bl_tui::{CommonResult, ZOrder};
