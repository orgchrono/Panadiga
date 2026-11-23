#![deny(unsafe_code)]

//! Tauri IPC command for guarded EH55 Upper NOR BGO FTL repair (`SA26..=SA70`, `CONFIRM_UPPER_NOR`).

use std::fs;
use std::path::PathBuf;

use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport, SerialPanamon1Transport};
use panadiga_core::re::eh55_bgo::{
    BgoRepairPlan, apply_bgo_repair_plan_live, build_bgo_repair_plan,
};
use tauri::Emitter;

use super::eh55_bgo_dto::{
    BgoRepairProgressEventDto, PicoEh55BgoRepairRequestDto, PicoEh55BgoRepairResponseDto,
    UPPER_NOR_CONFIRM_TOKEN, resolve_auto_backup_dir, run_exclusive_panamon1,
};
use crate::commands::constants::EVENT_BGO_REPAIR_PROGRESS;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::commands::frm_pack::{resolve_read_path, resolve_write_path};
use crate::commands::unbrick_dto::{opt_read_path, opt_trimmed};
use crate::state::PanadigaBackendState;

fn load_or_build_bgo_plan(
    req: &PicoEh55BgoRepairRequestDto,
) -> IpcResult<(BgoRepairPlan, Option<PathBuf>)> {
    if let Some(plan_raw) = opt_trimmed(&req.plan_path) {
        let p = resolve_read_path(plan_raw);
        let plan: BgoRepairPlan = serde_json::from_slice(
            &fs::read(&p).map_err(|e| PanadigaIpcError::io(format!("{e}")))?,
        )
        .map_err(|e| PanadigaIpcError::validation(format!("BgoRepairPlan JSON inválido: {e}")))?;
        return Ok((plan, None));
    }
    let nor_raw = opt_trimmed(&req.nor_path).ok_or_else(|| {
        PanadigaIpcError::validation("Informe norPath ou planPath para BGO repair.")
    })?;
    let nor_path = resolve_read_path(nor_raw);
    let nor = fs::read(&nor_path).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
    let ref_nor = opt_read_path(&req.reference_nor_path)
        .map(|rp| fs::read(&rp).map_err(|e| PanadigaIpcError::io(format!("{e}"))))
        .transpose()?;
    let plan = build_bgo_repair_plan(&nor, ref_nor.as_deref())
        .map_err(|e| PanadigaIpcError::classify(format!("Falha montando BgoRepairPlan: {e}")))?;
    Ok((plan, Some(nor_path)))
}

pub fn run_eh55_bgo_repair_with_transport<T: Panamon1Transport, F>(
    req: &PicoEh55BgoRepairRequestDto,
    transport: Option<T>,
    mut on_progress: F,
) -> IpcResult<PicoEh55BgoRepairResponseDto>
where
    F: FnMut(BgoRepairProgressEventDto),
{
    let (plan, src_nor_opt) = load_or_build_bgo_plan(req)?;
    let out_plan_path = opt_trimmed(&req.out_plan_path)
        .map(|raw| -> IpcResult<String> {
            let dst = resolve_write_path(raw)?;
            let json = serde_json::to_vec_pretty(&plan)
                .map_err(|e| PanadigaIpcError::internal(format!("{e}")))?;
            fs::write(&dst, json).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
            Ok(dst.to_string_lossy().into_owned())
        })
        .transpose()?;
    let cow_out_path = opt_trimmed(&req.cow_out_path)
        .map(|raw| -> IpcResult<String> {
            let dst = resolve_write_path(raw)?;
            if let Some(ref src) = src_nor_opt
                && src == &dst
            {
                return Err(PanadigaIpcError::validation(
                    "cowOutPath não pode sobrescrever norPath.",
                ));
            }
            let img = plan.repaired_nor_image.as_deref().ok_or_else(|| {
                PanadigaIpcError::validation(
                    "Plano sem repaired_nor_image; gere a partir de norPath.",
                )
            })?;
            fs::write(&dst, img).map_err(|e| PanadigaIpcError::io(format!("{e}")))?;
            Ok(dst.to_string_lossy().into_owned())
        })
        .transpose()?;

    if req.dry_run_only {
        return Ok(PicoEh55BgoRepairResponseDto {
            dry_run_only: true,
            sa0_to_sa3_locked_e05: true,
            plan,
            out_plan_path,
            cow_out_path,
            live_report: None,
        });
    }

    if opt_trimmed(&req.confirm_token) != Some(UPPER_NOR_CONFIRM_TOKEN) {
        return Err(PanadigaIpcError::validation(format!(
            "Bloqueio E05 Upper NOR: confirmToken deve ser '{}' para live repair (SA0..SA3 seguem travados).",
            UPPER_NOR_CONFIRM_TOKEN
        )));
    }
    let backup_dir = resolve_auto_backup_dir(opt_trimmed(&req.backup_dir), "eh55_bgo_backup")?;
    let t = transport
        .ok_or_else(|| PanadigaIpcError::validation("Transporte serial Pico obrigatório."))?;
    let mut client = Panamon1Client::new(t);
    let live_rep = apply_bgo_repair_plan_live(
        &mut client,
        &plan,
        &backup_dir,
        true,
        req.run_boot_trace_ms,
        &mut |p| {
            on_progress(BgoRepairProgressEventDto {
                stage: p.stage,
                current: p.current,
                total: p.total,
                detail: p.detail,
                telemetry: p.telemetry,
            });
        },
    )
    .map_err(|e| PanadigaIpcError::classify(format!("Falha aplicando BGO repair ao vivo: {e}")))?;

    Ok(PicoEh55BgoRepairResponseDto {
        dry_run_only: false,
        sa0_to_sa3_locked_e05: true,
        plan,
        out_plan_path,
        cow_out_path,
        live_report: Some(live_rep),
    })
}

/// Computes the normalized input reference bound into ephemeral `NorWrite` confirmation nonces.
pub fn normalize_bgo_nonce_input_ref(req: &PicoEh55BgoRepairRequestDto) -> String {
    opt_trimmed(&req.nor_path)
        .or_else(|| opt_trimmed(&req.plan_path))
        .unwrap_or("live_plan")
        .to_string()
}

/// Builds a CoW BGO FTL repair plan (`dryRunOnly=true`) or applies it live with `CONFIRM_UPPER_NOR`
/// (`SA0..SA3` permanently locked `E05`), emitting `"bgo://repair-progress"`.
#[tauri::command]
pub async fn pico_eh55_bgo_repair_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, PanadigaBackendState>,
    request: PicoEh55BgoRepairRequestDto,
) -> IpcResult<PicoEh55BgoRepairResponseDto> {
    if request.dry_run_only {
        return run_eh55_bgo_repair_with_transport::<SerialPanamon1Transport, _>(
            &request,
            None,
            |_| {},
        );
    }
    if opt_trimmed(&request.confirm_token) != Some(UPPER_NOR_CONFIRM_TOKEN) {
        return Err(PanadigaIpcError::validation(format!(
            "Bloqueio E05 Upper NOR: confirmToken deve ser '{}'.",
            UPPER_NOR_CONFIRM_TOKEN
        )));
    }
    let clean_port = opt_trimmed(&request.port_name)
        .ok_or_else(|| PanadigaIpcError::validation("portName obrigatório para BGO live repair."))?
        .to_string();
    let input_ref = normalize_bgo_nonce_input_ref(&request);
    super::bench_control::consume_operation_nonce_with_state(
        state.inner(),
        request.nonce_id.as_deref(),
        panadiga_core::bench::OpKind::NorWrite,
        "SA26..SA70",
        &input_ref,
        true,
    )?;
    let backend = state.inner().clone();
    let app_clone = app.clone();
    let req_clone = request.clone();
    run_exclusive_panamon1(backend, clean_port, "BGO repair", move |t| {
        run_eh55_bgo_repair_with_transport(&req_clone, Some(t), |ev| {
            let _ = app_clone.emit(EVENT_BGO_REPAIR_PROGRESS, &ev);
        })
    })
    .await
}
