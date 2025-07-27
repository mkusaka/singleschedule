use r3bl_tui::{
    ch, col, new_style, render_ops, render_tui_styled_texts_into, row, send_signal, throws_with_return,
    tui_color, tui_styled_text, tui_styled_texts, BoxedSafeComponent, Component, EventPropagation,
    FlexBox, FlexBoxId, GlobalData, HasFocus, InputEvent, Key, KeyPress, RenderOp, RenderPipeline,
    SpecialKey, SurfaceBounds, TerminalWindowMainThreadSignal,
};

use super::{AppSignal, State};

pub struct TaskListComponent {
    pub id: FlexBoxId,
}

impl TaskListComponent {
    pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
        Box::new(Self { id })
    }
}

impl Component<State, AppSignal> for TaskListComponent {
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
            let state = &mut global_data.state;
            let mut event_consumed = false;

            if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                // Check for character keys
                if let Key::Character(typed_char) = key {
                    match typed_char {
                        ' ' => {
                            event_consumed = true;
                            // Toggle task active state
                            let index = state.selected_index;
                            send_signal!(
                                global_data.main_thread_channel_sender,
                                TerminalWindowMainThreadSignal::ApplyAppSignal(
                                    AppSignal::ToggleTask(index)
                                )
                            );
                        }
                        'r' => {
                            event_consumed = true;
                            // Refresh tasks from storage
                            send_signal!(
                                global_data.main_thread_channel_sender,
                                TerminalWindowMainThreadSignal::ApplyAppSignal(
                                    AppSignal::RefreshTasks
                                )
                            );
                        }
                        _ => {}
                    }
                }

                // Check for special keys
                if let Key::SpecialKey(special_key) = key {
                    match special_key {
                        SpecialKey::Up => {
                            event_consumed = true;
                            if state.selected_index > 0 {
                                state.selected_index -= 1;
                            }
                        }
                        SpecialKey::Down => {
                            event_consumed = true;
                            if state.selected_index < state.tasks.len().saturating_sub(1) {
                                state.selected_index += 1;
                            }
                        }
                        SpecialKey::Enter => {
                            event_consumed = true;
                            // Toggle daemon for selected task
                            let selected_index = state.selected_index;
                            if let Some(task) = state.tasks.get(selected_index) {
                                let task_slug = task.slug.clone();
                                let is_active = task.active;
                                tokio::spawn(async move {
                                    let _ = if is_active {
                                        crate::cli::handle_stop(vec![task_slug], false).await
                                    } else {
                                        crate::cli::handle_start(vec![task_slug], false).await
                                    };
                                });
                            }
                        }
                        _ => {}
                    }
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
            let mut render_ops = render_ops!();
            let state = &global_data.state;

            // Use current_box properties for layout
            let box_origin_pos = current_box.style_adjusted_origin_pos;
            let box_bounds_size = current_box.style_adjusted_bounds_size;

            let mut row_index = row(0);

            // Header - use relative positioning from box origin
            let header_styled_texts = tui_styled_texts! {
                tui_styled_text!{
                    @style: new_style!(bold color_fg: {tui_color!(hex "#00BFFF")}),
                    @text: "SingleSchedule TUI"
                },
                tui_styled_text!{
                    @style: new_style!(dim color_fg: {tui_color!(hex "#888888")}),
                    @text: " - Task Manager"
                },
            };
            render_ops.push(RenderOp::MoveCursorPositionRelTo(
                box_origin_pos,
                col(0) + row_index,
            ));
            render_tui_styled_texts_into(&header_styled_texts, &mut render_ops);
            row_index += row(2); // Skip a line

            // Task list
            if state.tasks.is_empty() {
                let empty_text = tui_styled_texts! {
                    tui_styled_text!{
                        @style: new_style!(dim color_fg: {tui_color!(hex "#666666")}),
                        @text: "No tasks scheduled. Press 'a' to add a task."
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin_pos,
                    col(0) + row_index,
                ));
                render_tui_styled_texts_into(&empty_text, &mut render_ops);
            } else {
                // Column headers (adjusted for selection indicator)
                let headers = tui_styled_texts! {
                    tui_styled_text!{
                        @style: new_style!(bold underline color_fg: {tui_color!(hex "#CCCCCC")}),
                        @text: "  Status  Slug                 Cron                 Command                        Last Run"
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionRelTo(
                    box_origin_pos,
                    col(0) + row_index,
                ));
                render_tui_styled_texts_into(&headers, &mut render_ops);
                row_index += row(1);

                // Task rows
                let max_visible_rows = box_bounds_size
                    .row_height
                    .as_usize()
                    .saturating_sub(row_index.as_usize() + 1); // Account for header and current position
                let start_index = state.selected_index.saturating_sub(max_visible_rows / 2);
                let end_index = (start_index + max_visible_rows).min(state.tasks.len());

                for (i, task) in state.tasks[start_index..end_index].iter().enumerate() {
                    let abs_index = start_index + i;
                    let is_selected = abs_index == state.selected_index;

                    // Background for selected row
                    if is_selected {
                        // The actual content width is the sum of all formatted field widths:
                        // Indicator: 2, Status: 8, Slug: 20, Cron: 20, Command: 30, LastRun: 15 = 95 chars total
                        let content_width = 95;
                        
                        render_ops.push(RenderOp::MoveCursorPositionRelTo(
                            box_origin_pos,
                            col(0) + row_index,
                        ));
                        render_ops.push(RenderOp::SetBgColor(tui_color!(hex "#333366")));
                        render_ops.push(RenderOp::PaintTextWithAttributes(
                            " ".repeat(content_width.min(box_bounds_size.col_width.as_usize())).into(),
                            None,
                        ));
                    }

                    // Selection indicator
                    let selection_indicator = if is_selected { "▶ " } else { "  " };
                    
                    // Status icon
                    let status_icon = if task.active { "●" } else { "○" };
                    let status_color = if task.active {
                        tui_color!(hex "#00FF00")
                    } else {
                        tui_color!(hex "#FF0000")
                    };

                    // Last run time
                    let last_run = task
                        .last_run
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Never".to_string());

                    // Command display (truncate if too long)
                    let command_display = if task.command.len() > 27 {
                        format!("{}...", &task.command[..27])
                    } else {
                        task.command.clone()
                    };

                    // Render task row with selection indicator
                    let task_text = tui_styled_texts! {
                        tui_styled_text!{
                            @style: new_style!(bold color_fg: {tui_color!(hex "#FFFF00")}),
                            @text: selection_indicator
                        },
                        tui_styled_text!{
                            @style: new_style!(color_fg: {status_color}),
                            @text: format!("{:<8}", status_icon)
                        },
                        tui_styled_text!{
                            @style: new_style!(color_fg: {tui_color!(hex "#00FFFF")}),
                            @text: format!("{:<20}", task.slug)
                        },
                        tui_styled_text!{
                            @style: new_style!(color_fg: {tui_color!(hex "#FFFF00")}),
                            @text: format!("{:<20}", task.cron)
                        },
                        tui_styled_text!{
                            @style: new_style!(color_fg: {tui_color!(hex "#FFFFFF")}),
                            @text: format!("{:<30}", command_display)
                        },
                        tui_styled_text!{
                            @style: new_style!(color_fg: {tui_color!(hex "#FF00FF")}),
                            @text: format!("{:<15}", last_run)
                        },
                    };

                    render_ops.push(RenderOp::MoveCursorPositionRelTo(
                        box_origin_pos,
                        col(0) + row_index,
                    ));
                    render_tui_styled_texts_into(&task_text, &mut render_ops);

                    // Reset background color
                    if is_selected {
                        render_ops.push(RenderOp::ResetColor);
                    }

                    row_index += row(1);
                }

                // Scroll indicator if needed
                if state.tasks.len() > max_visible_rows {
                    let scroll_info =
                        format!(" ({}/{}) ", state.selected_index + 1, state.tasks.len());
                    let scroll_text = tui_styled_texts! {
                        tui_styled_text!{
                            @style: new_style!(dim color_fg: {tui_color!(hex "#888888")}),
                            @text: scroll_info
                        },
                    };
                    // Position at bottom right of the box
                    let scroll_width = scroll_info.len();
                    let bottom_row = box_bounds_size.row_height.convert_to_row_index();
                    let right_col = box_bounds_size
                        .col_width
                        .as_usize()
                        .saturating_sub(scroll_width);
                    render_ops.push(RenderOp::MoveCursorPositionRelTo(
                        box_origin_pos,
                        col(right_col) + bottom_row,
                    ));
                    render_tui_styled_texts_into(&scroll_text, &mut render_ops);
                }
            }

            // Show message if any (above hints)
            if let Some(message) = &state.message {
                if !message.is_empty() {
                    row_index += row(2);
                    
                    if row_index.as_usize() < box_bounds_size.row_height.as_usize() {
                        let message_text = tui_styled_texts! {
                            tui_styled_text!{
                                @style: new_style!(bold color_fg: {tui_color!(hex "#00FF00")} color_bg: {tui_color!(hex "#1E1E2E")}),
                                @text: format!(" {} ", message)
                            },
                        };
                        
                        // Center the message
                        let message_width = message_text.display_width();
                        let message_col = if box_bounds_size.col_width > message_width {
                            ((box_bounds_size.col_width - message_width) / ch(2)).as_usize()
                        } else {
                            0
                        };
                        
                        render_ops.push(RenderOp::MoveCursorPositionRelTo(
                            box_origin_pos,
                            col(message_col) + row_index,
                        ));
                        render_tui_styled_texts_into(&message_text, &mut render_ops);
                    }
                }
            }

            // Add hints at the bottom of the table view
            {
                row_index += row(1); // Add some spacing
                
                // Ensure we don't render beyond the box bounds
                if row_index.as_usize() < box_bounds_size.row_height.as_usize() {
                    let hints_text = tui_styled_texts! {
                        tui_styled_text!{
                            @style: new_style!(dim color_fg: {tui_color!(hex "#888888")}),
                            @text: "Hints: "
                        },
                        tui_styled_text!{
                            @style: new_style!(bold color_fg: {tui_color!(hex "#AAAAAA")}),
                            @text: "ESC/x: Exit | a: Add Task | d: Delete | Space: Toggle | r: Refresh"
                        },
                    };
                    
                    render_ops.push(RenderOp::MoveCursorPositionRelTo(
                        box_origin_pos,
                        col(0) + row_index,
                    ));
                    render_tui_styled_texts_into(&hints_text, &mut render_ops);
                }
            }

            let mut render_pipeline = RenderPipeline::default();
            render_pipeline.push(ZOrder::Normal, render_ops);
            render_pipeline
        })
    }
}

use r3bl_tui::{CommonResult, ZOrder};
