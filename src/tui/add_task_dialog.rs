use r3bl_tui::{
    col, new_style, render_ops, render_tui_styled_texts_into, row, send_signal, throws_with_return,
    tui_color, tui_styled_text, tui_styled_texts, BoxedSafeComponent, Component, EventPropagation,
    FlexBox, FlexBoxId, GlobalData, HasFocus, InputEvent, Key, KeyPress, RenderOp, RenderPipeline,
    SpecialKey, SurfaceBounds, TerminalWindowMainThreadSignal,
};

use super::{AppSignal, State};

pub struct AddTaskDialog {
    pub id: FlexBoxId,
}

impl AddTaskDialog {
    pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
        Box::new(Self { id })
    }
}

impl Component<State, AppSignal> for AddTaskDialog {
    fn reset(&mut self) {
        // Nothing to reset (new task input reset happens in App)
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
            let state = &mut global_data.state;
            let mut event_consumed = false;

            if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                match key {
                    Key::SpecialKey(SpecialKey::Esc) => {
                        event_consumed = true;
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::CloseDialog)
                        );
                    }
                    Key::SpecialKey(SpecialKey::Tab) => {
                        event_consumed = true;
                        state.new_task.current_field = (state.new_task.current_field + 1) % 3;
                    }
                    Key::SpecialKey(SpecialKey::Enter) => {
                        event_consumed = true;
                        if state.new_task.current_field < 2 {
                            state.new_task.current_field += 1;
                        } else {
                            // Try to create task
                            if let Some(task) = state.new_task.create_task() {
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::AddTask(task)
                                    )
                                );
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::ShowMessage(
                                            "Task added successfully".to_string()
                                        )
                                    )
                                );
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::CloseDialog
                                    )
                                );
                            } else {
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::ShowMessage(
                                            "Invalid input. Please check all fields.".to_string()
                                        )
                                    )
                                );
                            }
                        }
                    }
                    Key::SpecialKey(SpecialKey::Backspace) => {
                        event_consumed = true;
                        state.new_task.handle_backspace();
                    }
                    Key::Character(ch) => {
                        event_consumed = true;
                        state.new_task.handle_char(ch);
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
            if !global_data.state.show_add_dialog {
                return Ok(RenderPipeline::default());
            }

            let mut render_pipeline = RenderPipeline::default();
            let mut render_ops = render_ops!();
            let state = &global_data.state;

            // Use box bounds for dialog dimensions
            let box_bounds_size = current_box.style_adjusted_bounds_size;

            // Fixed dialog size
            let dialog_width = 60.min(box_bounds_size.col_width.as_usize());
            let dialog_height = 12.min(box_bounds_size.row_height.as_usize());

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

            // Origin position of the box
            let box_origin = current_box.style_adjusted_origin_pos;

            // Draw dialog background
            for row_offset in 0..dialog_height {
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x) + row(y + row_offset),
                ));
                render_ops.push(RenderOp::SetBgColor(tui_color!(hex "#1E1E2E")));
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    " ".repeat(dialog_width).into(),
                    None,
                ));
            }

            // Draw border
            // Top border
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x) + row(y),
            ));
            render_ops.push(RenderOp::SetFgColor(tui_color!(hex "#00BFFF")));
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
                    @style: new_style!(bold color_fg: {tui_color!(hex "#00FFFF")} color_bg: {tui_color!(hex "#1E1E2E")}),
                    @text: "Add New Task"
                },
            };
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x + 2) + row(y + 1),
            ));
            render_tui_styled_texts_into(&title_text, &mut render_ops);

            // Form fields
            let fields = [
                ("Slug:", &state.new_task.slug, 0),
                ("Cron:", &state.new_task.cron, 1),
                ("Command:", &state.new_task.command, 2),
            ];

            for (i, (label, value, field_index)) in fields.iter().enumerate() {
                let field_y = y + 3 + (i * 2);
                let is_active = state.new_task.current_field == *field_index;

                // Field label
                let label_text = tui_styled_texts! {
                    tui_styled_text!{
                        @style: new_style!(color_fg: {tui_color!(hex "#AAAAAA")} color_bg: {tui_color!(hex "#1E1E2E")}),
                        @text: label
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(x + 2) + row(field_y),
                ));
                render_tui_styled_texts_into(&label_text, &mut render_ops);

                // Field value
                let field_x = x + 12;
                let field_width = dialog_width - 14;
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin,
                    col(field_x) + row(field_y),
                ));

                if is_active {
                    render_ops.push(RenderOp::SetBgColor(tui_color!(hex "#333366")));
                }

                let display_value = if value.len() > field_width {
                    &value[value.len().saturating_sub(field_width)..]
                } else {
                    value
                };

                let value_text = tui_styled_texts! {
                    tui_styled_text!{
                        @style: new_style!(color_fg: {tui_color!(hex "#FFFFFF")} color_bg: {
                            if is_active {
                                tui_color!(hex "#333366")
                            } else {
                                tui_color!(hex "#1E1E2E")
                            }
                        }),
                        @text: format!("{:<width$}", display_value, width = field_width)
                    },
                };
                render_tui_styled_texts_into(&value_text, &mut render_ops);

                // Show cursor for active field
                if is_active {
                    // Calculate cursor position based on actual value length
                    let cursor_offset = if value.len() > field_width {
                        field_width // Cursor at the end of the visible field when text is scrolled
                    } else {
                        value.len() // Cursor after the actual text when it fits
                    };

                    render_ops.push(RenderOp::MoveCursorPositionRelTo(
                        box_origin,
                        col(field_x + cursor_offset) + row(field_y),
                    ));
                    render_ops.push(RenderOp::SetFgColor(tui_color!(hex "#FFFFFF")));
                    render_ops.push(RenderOp::SetBgColor(tui_color!(hex "#FFFFFF")));
                    render_ops.push(RenderOp::PaintTextWithAttributes(" ".into(), None));
                }

                render_ops.push(RenderOp::ResetColor);
            }

            // Instructions
            let instructions = tui_styled_texts! {
                tui_styled_text!{
                    @style: new_style!(dim color_fg: {tui_color!(hex "#666666")} color_bg: {tui_color!(hex "#1E1E2E")}),
                    @text: "Tab: Next field | Enter: Submit | Esc: Cancel"
                },
            };
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin,
                col(x + 2) + row(y + dialog_height - 2),
            ));
            render_tui_styled_texts_into(&instructions, &mut render_ops);

            render_ops.push(RenderOp::ResetColor);
            render_pipeline.push(ZOrder::Glass, render_ops);
            render_pipeline
        })
    }
}

use r3bl_tui::{CommonResult, ZOrder};
