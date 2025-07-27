use r3bl_tui::{
    box_end, box_start, ch, col, new_style, render_component_in_current_box,
    req_size_pc, row, surface, throws, throws_with_return, tui_color,
    tui_stylesheet, App, BoxedSafeApp, CommonResult,
    ComponentRegistry, ComponentRegistryMap, ContainsResult, EventPropagation, FlexBox, FlexBoxId,
    GlobalData, HasFocus, InputEvent, Key, KeyPress, LayoutDirection, LayoutManagement,
    PerformPositioningAndSizing, RenderPipeline, SpecialKey, Surface,
    SurfaceBounds, SurfaceProps, SurfaceRender, TuiStylesheet,
};

use super::{
    add_task_dialog::AddTaskDialog, delete_confirm_dialog::DeleteConfirmDialog,
    task_list_component::TaskListComponent, AppSignal, State,
};

// Constants for the component IDs
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 1,
    TaskList = 2,
    AddTaskDialog = 3,
    DeleteConfirmDialog = 4,
}

impl From<Id> for u8 {
    fn from(id: Id) -> u8 {
        id as u8
    }
}

impl From<Id> for FlexBoxId {
    fn from(id: Id) -> FlexBoxId {
        FlexBoxId::new(id)
    }
}

#[derive(Default)]
pub struct AppMain {
    _phantom: std::marker::PhantomData<(State, AppSignal)>,
}

impl AppMain {
    pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
        let it = Self::default();
        Box::new(it)
    }
}

impl App for AppMain {
    type S = State;
    type AS = AppSignal;

    fn app_init(
        &mut self,
        component_registry_map: &mut ComponentRegistryMap<Self::S, Self::AS>,
        has_focus: &mut HasFocus,
    ) {
        Self::init_component_registry(component_registry_map, has_focus);
    }

    fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<State, AppSignal>,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        // If a dialog is open, handle it specially
        if global_data.state.show_add_dialog || global_data.state.show_delete_dialog {
            // Check if it's ESC key first (to close dialog)
            if let InputEvent::Keyboard(KeyPress::Plain { key: Key::SpecialKey(SpecialKey::Esc) }) = &input_event {
                global_data.state.show_add_dialog = false;
                global_data.state.show_delete_dialog = false;
                has_focus.set_id(FlexBoxId::from(Id::TaskList));
                return Ok(EventPropagation::ConsumedRender);
            }
            
            // Route other events to the dialog
            return ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry_map,
                has_focus,
            );
        }

        // Handle global shortcuts only when no dialog is open
        if let InputEvent::Keyboard(key_press) = &input_event {
            match key_press {
                KeyPress::WithModifiers { key, mask } => {
                    // Ctrl+Q is handled by exit_keys in main_event_loop
                    _ = (key, mask); // Avoid unused variable warnings
                }
                KeyPress::Plain { key } => {
                    match key {
                        Key::Character('a') => {
                            // Show add task dialog
                            global_data.state.show_add_dialog = true;
                            has_focus.set_id(FlexBoxId::from(Id::AddTaskDialog));
                            return Ok(EventPropagation::ConsumedRender);
                        }
                        Key::Character('d') => {
                            // Show delete confirmation dialog
                            if !global_data.state.tasks.is_empty() {
                                global_data.state.show_delete_dialog = true;
                                has_focus.set_id(FlexBoxId::from(Id::DeleteConfirmDialog));
                                return Ok(EventPropagation::ConsumedRender);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Route to focused component
        ComponentRegistry::route_event_to_focused_component(
            global_data,
            input_event,
            component_registry_map,
            has_focus,
        )
    }

    fn app_handle_signal(
        &mut self,
        signal: &AppSignal,
        global_data: &mut GlobalData<State, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            match signal {
                AppSignal::RefreshTasks => {
                    // Reload tasks from storage
                    if let Ok(loaded_state) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(State::load_from_storage())
                    }) {
                        global_data.state.tasks = loaded_state.tasks;
                        global_data.state.message = Some("Tasks refreshed successfully!".to_string());
                    } else {
                        global_data.state.message = Some("Failed to refresh tasks".to_string());
                    }
                    EventPropagation::ConsumedRender
                }
                AppSignal::SaveState => {
                    // Save current state to storage
                    let _ = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(global_data.state.save_to_storage())
                    });
                    EventPropagation::ConsumedRender
                }
                AppSignal::ToggleTask(index) => {
                    if let Some(task) = global_data.state.tasks.get_mut(*index) {
                        task.active = !task.active;
                        let _ = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(global_data.state.save_to_storage())
                        });
                    }
                    EventPropagation::ConsumedRender
                }
                AppSignal::DeleteTask(index) => {
                    if *index < global_data.state.tasks.len() {
                        global_data.state.tasks.remove(*index);
                        if global_data.state.selected_index >= global_data.state.tasks.len()
                            && global_data.state.selected_index > 0
                        {
                            global_data.state.selected_index = global_data.state.tasks.len() - 1;
                        }
                        let _ = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(global_data.state.save_to_storage())
                        });
                    }
                    EventPropagation::ConsumedRender
                }
                AppSignal::AddTask(task) => {
                    global_data.state.tasks.push(task.clone());
                    let _ = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(global_data.state.save_to_storage())
                    });
                    EventPropagation::ConsumedRender
                }
                AppSignal::CloseDialog => {
                    global_data.state.show_add_dialog = false;
                    global_data.state.show_delete_dialog = false;
                    has_focus.set_id(FlexBoxId::from(Id::TaskList));
                    EventPropagation::ConsumedRender
                }
                AppSignal::ShowMessage(msg) => {
                    if msg.is_empty() {
                        global_data.state.message = None;
                    } else {
                        global_data.state.message = Some(msg.clone());
                        
                        // Auto-clear message after 3 seconds
                        let sender = global_data.main_thread_channel_sender.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            let _ = sender.send(
                                r3bl_tui::TerminalWindowMainThreadSignal::ApplyAppSignal(
                                    AppSignal::ShowMessage("".to_string())
                                )
                            ).await;
                        });
                    }
                    
                    EventPropagation::ConsumedRender
                }
            }
        });
    }

    fn app_render(
        &mut self,
        global_data: &mut GlobalData<State, AppSignal>,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let window_size = global_data.window_size;

            // Create the main surface
            let mut surface = {
                let mut it = surface!(stylesheet: create_stylesheet()?);

                it.surface_start(SurfaceProps {
                    pos: col(0) + row(0),
                    size: window_size,
                })?;

                // Render main container
                ContainerSurfaceRender { _app: self }.render_in_surface(
                    &mut it,
                    global_data,
                    component_registry_map,
                    has_focus,
                )?;

                it.surface_end()?;
                it
            };

            // Message rendering removed - now handled within TaskListComponent

            // Render modal dialogs
            if global_data.state.show_add_dialog {
                render_add_dialog(
                    &mut surface.render_pipeline,
                    global_data,
                    component_registry_map,
                    has_focus,
                )?;
            }

            if global_data.state.show_delete_dialog {
                render_delete_dialog(
                    &mut surface.render_pipeline,
                    global_data,
                    component_registry_map,
                    has_focus,
                )?;
            }

            surface.render_pipeline
        });
    }
}

impl AppMain {
    pub fn init_component_registry(
        map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        // Create and register task list component
        let task_list_id = FlexBoxId::from(Id::TaskList);
        if let ContainsResult::DoesNotContain = ComponentRegistry::contains(map, task_list_id) {
            let component = TaskListComponent::new_boxed(task_list_id);
            ComponentRegistry::put(map, task_list_id, component);
        }

        // Create and register add task dialog component
        let add_dialog_id = FlexBoxId::from(Id::AddTaskDialog);
        if let ContainsResult::DoesNotContain = ComponentRegistry::contains(map, add_dialog_id) {
            let component = AddTaskDialog::new_boxed(add_dialog_id);
            ComponentRegistry::put(map, add_dialog_id, component);
        }

        // Create and register delete confirm dialog component
        let delete_dialog_id = FlexBoxId::from(Id::DeleteConfirmDialog);
        if let ContainsResult::DoesNotContain = ComponentRegistry::contains(map, delete_dialog_id) {
            let component = DeleteConfirmDialog::new_boxed(delete_dialog_id);
            ComponentRegistry::put(map, delete_dialog_id, component);
        }

        // Set initial focus
        if has_focus.get_id().is_none() {
            has_focus.set_id(task_list_id);
        }
    }
}

struct ContainerSurfaceRender<'a> {
    _app: &'a mut AppMain,
}

impl SurfaceRender<State, AppSignal> for ContainerSurfaceRender<'_> {
    fn render_in_surface(
        &mut self,
        surface: &mut Surface,
        global_data: &mut GlobalData<State, AppSignal>,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<()> {
        throws!({
            let component_id = FlexBoxId::from(Id::TaskList);

            // Layout task list component
            box_start!(
                in: surface,
                id: component_id,
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 100, height: 100),
                styles: [component_id]
            );
            render_component_in_current_box!(
                in: surface,
                component_id: component_id,
                from: component_registry_map,
                global_data: global_data,
                has_focus: has_focus
            );
            box_end!(in: surface);
        })
    }
}

fn create_stylesheet() -> CommonResult<TuiStylesheet> {
    throws_with_return!({
        tui_stylesheet! {
            new_style!(id: {Id::Container} padding: {ch(1)}),
            new_style!(id: {Id::TaskList} padding: {ch(1)} color_bg: {tui_color!(23, 23, 28)}),
            new_style!(id: {Id::AddTaskDialog} padding: {ch(2)} color_bg: {tui_color!(30, 30, 40)}),
            new_style!(id: {Id::DeleteConfirmDialog} padding: {ch(2)} color_bg: {tui_color!(50, 30, 30)})
        }
    })
}

// Message and status bar rendering removed - now handled within TaskListComponent

// Dialog rendering functions
fn render_add_dialog(
    pipeline: &mut RenderPipeline,
    global_data: &mut GlobalData<State, AppSignal>,
    component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    has_focus: &mut HasFocus,
) -> CommonResult<()> {
    if let Some(component) = ComponentRegistry::try_to_get_component_by_id(
        component_registry_map,
        FlexBoxId::from(Id::AddTaskDialog),
    ) {
        let window_size = global_data.window_size;
        let surface_bounds = SurfaceBounds {
            origin_pos: col(0) + row(0),
            box_size: window_size,
        };
        let current_box = FlexBox {
            id: FlexBoxId::from(Id::AddTaskDialog),
            dir: LayoutDirection::Vertical,
            origin_pos: col(0) + row(0),
            bounds_size: window_size,
            style_adjusted_origin_pos: col(0) + row(0),
            style_adjusted_bounds_size: window_size,
            requested_size_percent: req_size_pc!(width: 100, height: 100),
            insertion_pos_for_next_box: None,
            maybe_computed_style: None,
        };
        let component_pipeline =
            component.render(global_data, current_box, surface_bounds, has_focus)?;
        // Merge component pipeline into main pipeline
        for (z_order, render_ops_vec) in component_pipeline.iter() {
            for render_op in render_ops_vec.iter() {
                pipeline.push(*z_order, render_op.clone());
            }
        }
    }
    Ok(())
}

fn render_delete_dialog(
    pipeline: &mut RenderPipeline,
    global_data: &mut GlobalData<State, AppSignal>,
    component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    has_focus: &mut HasFocus,
) -> CommonResult<()> {
    if let Some(component) = ComponentRegistry::try_to_get_component_by_id(
        component_registry_map,
        FlexBoxId::from(Id::DeleteConfirmDialog),
    ) {
        let window_size = global_data.window_size;
        let surface_bounds = SurfaceBounds {
            origin_pos: col(0) + row(0),
            box_size: window_size,
        };
        let current_box = FlexBox {
            id: FlexBoxId::from(Id::DeleteConfirmDialog),
            dir: LayoutDirection::Vertical,
            origin_pos: col(0) + row(0),
            bounds_size: window_size,
            style_adjusted_origin_pos: col(0) + row(0),
            style_adjusted_bounds_size: window_size,
            requested_size_percent: req_size_pc!(width: 100, height: 100),
            insertion_pos_for_next_box: None,
            maybe_computed_style: None,
        };
        let component_pipeline =
            component.render(global_data, current_box, surface_bounds, has_focus)?;
        // Merge component pipeline into main pipeline
        for (z_order, render_ops_vec) in component_pipeline.iter() {
            for render_op in render_ops_vec.iter() {
                pipeline.push(*z_order, render_op.clone());
            }
        }
    }
    Ok(())
}
