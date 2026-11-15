"""
Tools: Panadiga Professional Engineering Drawings & Schematics Generator
Generates standards-compliant electrical schematics (IEEE Std 315 / IEC 60617)
and precise mechanical connector layout drawings for JST ZR B6B-ZR-3R4 / P7501.
Supports Canonical English (default) and Portuguese (fallback).
"""

from __future__ import annotations

import os
from pathlib import Path
import sys

_REPO_ROOT = str(Path(__file__).resolve().parents[2])
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

import matplotlib.patches as patches

from scripts.dev.cad_primitives import (
    create_drawing_canvas as create_drawing_canvas,
    draw_annotate,
    draw_chassis_ground as draw_chassis_ground,
    draw_ieee_resistor as draw_ieee_resistor,
    draw_signal_ground as draw_signal_ground,
    draw_text,
    finalize_and_save_drawing as finalize_and_save_drawing,
    plot_line,
    render_vertical_conductive_vias,
    setup_title_block as setup_title_block,
)
from scripts.dev.schematic_sheet import generate_electrical_schematic as generate_electrical_schematic


def generate_mechanical_cad_drawing(output_path: str, lang: str = "en") -> None:
    """Generates precise mechanical engineering CAD drawing of JST ZR B6B-ZR-3R4 header and PCB layout."""
    fig, ax = create_drawing_canvas()

    is_en = lang == "en"
    header_org = "PANADIGA BENCH LAB — MECHANICAL CAD SPECIFICATION" if is_en else "PANADIGA BENCH LAB — ESPECIFICAÇÃO CAD MECÂNICA"
    header_sub = "JST ZR 1.5mm Pitch"
    title_text = "TITLE: JST ZR B6B-ZR-3R4 Pinout & Latch Detail" if is_en else "TÍTULO: Pinagem e Detalhes da Trava JST ZR B6B-ZR-3R4"
    app_text = "APPLICATION: VEP79138 Test Port P7501 Assembly"
    rev_text = "REV: 2.5 (Canonical Manual Pinout 1..6)"
    date_scale = "UNITS: mm | SCALE: NTS"

    main_hdr = (
        "MECHANICAL CAD SPECIFICATION OF JST ZR (B6B-ZR-3R4) & PCB TOPOGRAPHY"
        if is_en
        else "DETALHAMENTO MECÂNICO DO CONECTOR JST ZR (B6B-ZR-3R4) E TOPOGRAFIA DE PCB"
    )
    sub_hdr = (
        "Latch Geometry, Thick vs Thin Shroud Wall, Pin 1 Reference & Canonical Panasonic Schematic Pinout"
        if is_en
        else "Geometria da Trava Plástica, Parede Grossa vs Fina, Chanfro de Polarização e Pinagem Canônica Panasonic"
    )

    setup_title_block(
        ax,
        header_org,
        header_sub,
        title_text,
        app_text,
        rev_text,
        date_scale,
        sheet_title=main_hdr,
        sheet_subtitle=sub_hdr,
    )

    sec1_hdr = "TOP VIEW: JST ZR HEADER (B6B-ZR-3R4) ON VEP79138 PCB" if is_en else "VISTA SUPERIOR: CONECTOR JST ZR (B6B-ZR-3R4) NA PLACA VEP79138"
    draw_text(ax, 4.8, 8.20, sec1_hdr, fontsize=8.5, fontweight="bold", ha="center", fontfamily="monospace")

    shroud = patches.Rectangle((1.0, 4.7), 7.6, 3.0, edgecolor="#000000", facecolor="#f8fafc", linewidth=2.0)
    ax.add_patch(shroud)

    back_wall = patches.Rectangle((1.0, 7.1), 7.6, 0.6, edgecolor="#000000", facecolor="#cbd5e1", linewidth=1.2)
    ax.add_patch(back_wall)
    bw_txt = (
        "REINFORCED REAR WALL (THICK 0.60mm) [TOWARDS BOARD INTERIOR]"
        if is_en
        else "PAREDE TRASEIRA REFORÇADA (GROSSA 0.60mm) [VOLTADA P/ O INTERIOR DA PLACA]"
    )
    draw_text(ax, 4.8, 7.4, bw_txt, fontsize=6.6, fontweight="bold", ha="center", va="center", fontfamily="sans-serif")

    front_wall = patches.Rectangle((1.0, 4.7), 7.6, 0.45, edgecolor="#000000", facecolor="#e2e8f0", linewidth=1.2)
    ax.add_patch(front_wall)

    lock_notch = patches.Rectangle((3.6, 4.65), 2.4, 0.55, edgecolor="#b91c1c", facecolor="#fee2e2", linewidth=1.5)
    ax.add_patch(lock_notch)
    notch_txt = "LOCKING NOTCH\n(FRONT BOARD EDGE)" if is_en else "JANELA DE TRAVA\n(BORDA FRONTAL PCB)"
    draw_text(ax, 4.8, 4.92, notch_txt, fontsize=5.2, fontweight="bold", color="#b91c1c", ha="center", va="center", fontfamily="sans-serif", linespacing=1.05)

    plot_line(ax, [1.6, 1.6], [4.7, 5.4], color="#000000", linewidth=1.5)
    plot_line(ax, [8.0, 8.0], [4.7, 5.4], color="#000000", linewidth=1.5)
    draw_text(ax, 1.6, 4.4, "KEY A" if is_en else "GUIA A", fontsize=6.5, fontweight="bold", ha="center", fontfamily="monospace")
    draw_text(ax, 8.0, 4.4, "KEY B" if is_en else "GUIA B", fontsize=6.5, fontweight="bold", ha="center", fontfamily="monospace")

    chamfer = patches.Polygon([[1.0, 7.1], [1.5, 7.7], [1.0, 7.7]], closed=True, edgecolor="#b91c1c", facecolor="#fecaca", linewidth=1.5)
    ax.add_patch(chamfer)
    ch_lbl = "45° BODY BEVEL (PIN 6 / UARTP2M_T PAD)" if is_en else "CHANFRO 45° (PAD PINO 6 / UARTP2M_T)"
    draw_text(ax, 1.0, 7.88, ch_lbl, fontsize=6.8, fontweight="bold", color="#b91c1c", ha="left", fontfamily="monospace")

    pin_xs = [2.05, 3.15, 4.25, 5.35, 6.45, 7.55]
    pin_data = [
        ("Pin 6" if is_en else "Pino 6", "UARTP2M_T", "(AM34 TX->GP2)", False, True),
        ("Pin 5" if is_en else "Pino 5", "NSW5V", "(Hi-Z Only!)", False, False),
        ("Pin 4" if is_en else "Pino 4", "GND", "(0.0Ω Chassis)" if is_en else "(0.0Ω Chassi)", True, False),
        ("Pin 3" if is_en else "Pino 3", "UARTM2P_T", "(GP4->Q7514)", False, False),
        ("Pin 2" if is_en else "Pino 2", "XRESET", "(Open-Drain)", False, False),
        ("Pin 1" if is_en else "Pino 1", "SW_UART", "(XFWE/GP26)", False, False),
    ]

    for i, px in enumerate(pin_xs):
        p_num, sig_main, sig_sub, is_gnd, is_square = pin_data[i]
        if is_square:
            pad_rect = patches.Rectangle((px - 0.25, 5.9), 0.5, 0.5, edgecolor="#b91c1c", facecolor="#fee2e2", linewidth=1.8)
            ax.add_patch(pad_rect)
            pad_txt = "SQUARE PAD\n(PCB PIN 6)" if is_en else "PAD QUADRADO\n(PINO 6 PCB)"
            draw_text(ax, px, 6.62, pad_txt, fontsize=6.0, fontweight="bold", color="#b91c1c", ha="center", fontfamily="monospace")
        else:
            pad_color = "#dcfce7" if is_gnd else "#ffffff"
            edge_c = "#15803d" if is_gnd else "#000000"
            pad_circ = patches.Circle((px, 6.15), 0.25, edgecolor=edge_c, facecolor=pad_color, linewidth=1.5)
            ax.add_patch(pad_circ)

        plot_line(ax, px, 6.15, marker=".", markersize=6, color="#000000")
        draw_text(ax, px, 5.45, p_num.split()[-1], fontsize=8, fontweight="bold", ha="center", va="center", fontfamily="monospace")
        draw_text(ax, px, 3.95, p_num, fontsize=6.5, fontweight="bold", ha="center", va="top", fontfamily="monospace")
        sig_color = "#15803d" if is_gnd else "#000000"
        draw_text(ax, px, 3.15, f"{sig_main}\n{sig_sub}", fontsize=6.8, fontweight="bold", color=sig_color, ha="center", va="top", fontfamily="monospace")

    draw_annotate(ax, "", xy=(3.15, 6.15), xytext=(2.05, 6.15), arrowprops={"arrowstyle": "<->", "color": "#2563eb", "lw": 1.2})
    draw_text(ax, 2.6, 6.25, "p=1.50mm", fontsize=6.8, color="#2563eb", ha="center", fontweight="bold", fontfamily="monospace")

    gnd_box = patches.Rectangle((3.80, 2.85), 0.90, 4.95, edgecolor="#15803d", facecolor="none", linewidth=1.8, linestyle="--")
    ax.add_patch(gnd_box)

    sec2_hdr = "REAL TOPOGRAPHY ON VEP79138 PCB" if is_en else "TOPOGRAFIA REAL NA PCB VEP79138"
    draw_text(ax, 12.425, 8.20, sec2_hdr, fontsize=8.5, fontweight="bold", ha="center", fontfamily="monospace")

    pcb_ctx = patches.Rectangle((9.5, 3.7), 5.85, 4.0, edgecolor="#000000", facecolor="#f1f5f9", linewidth=1.5)
    ax.add_patch(pcb_ctx)

    f10_sock = patches.Rectangle((9.70, 4.60), 1.35, 2.25, edgecolor="#000000", facecolor="#e2e8f0", linewidth=1.2)
    ax.add_patch(f10_sock)
    draw_text(ax, 10.375, 7.00, "FP50003\n(Flat 10 Vias)", fontsize=6.8, fontweight="bold", ha="center", fontfamily="monospace")
    render_vertical_conductive_vias(ax, [9.82 + i * 0.1233 for i in range(10)])

    p75_t1 = "P7501 [B6B-ZR-3R4]"
    p75_t2 = '"TO WRITE JIGU" (MALE)' if is_en else '"TO WRITE JIGU" (MACHO)'
    draw_text(ax, 12.425, 7.30, p75_t1, fontsize=7.0, fontweight="bold", color="#b91c1c", ha="center", fontfamily="monospace")
    draw_text(ax, 12.425, 7.02, p75_t2, fontsize=6.2, fontweight="bold", color="#b91c1c", ha="center", fontfamily="monospace")

    shroud_ctx = patches.Rectangle((11.25, 4.60), 2.35, 2.25, edgecolor="#b91c1c", facecolor="#f8fafc", linewidth=1.6)
    ax.add_patch(shroud_ctx)

    rear_wall = patches.Rectangle((11.25, 6.45), 2.35, 0.40, edgecolor="#b91c1c", facecolor="#cbd5e1", linewidth=1.0)
    ax.add_patch(rear_wall)
    wall_txt = "THICK REAR WALL (INSIDE)" if is_en else "PAREDE GROSSA (MIOLO)"
    draw_text(ax, 12.425, 6.65, wall_txt, fontsize=5.3, fontweight="bold", ha="center", va="center", fontfamily="sans-serif")

    front_wall_ctx = patches.Rectangle((11.25, 4.60), 2.35, 0.30, edgecolor="#b91c1c", facecolor="#e2e8f0", linewidth=1.0)
    ax.add_patch(front_wall_ctx)

    notch = patches.Rectangle((12.125, 4.55), 0.60, 0.35, edgecolor="#b91c1c", facecolor="#fee2e2", linewidth=1.2)
    ax.add_patch(notch)
    notch_lbl = "LATCH" if is_en else "TRAVA"
    draw_text(ax, 12.425, 4.72, notch_lbl, fontsize=5.3, fontweight="bold", color="#b91c1c", ha="center", va="center", fontfamily="monospace")

    chamfer_ctx = patches.Polygon([[11.25, 6.45], [11.60, 6.85], [11.25, 6.85]], closed=True, edgecolor="#b91c1c", facecolor="#fecaca", linewidth=1.0)
    ax.add_patch(chamfer_ctx)

    cavity = patches.Rectangle((11.38, 4.90), 2.09, 1.55, edgecolor="#94a3b8", facecolor="#ffffff", linewidth=0.8)
    ax.add_patch(cavity)

    for i in range(6):
        px = 11.55 + i * 0.35
        pin_idx = 6 - i
        is_gnd_pin = pin_idx == 4
        is_sq_pin = pin_idx == 6

        if is_sq_pin:
            post_rect = patches.Rectangle((px - 0.11, 5.39), 0.22, 0.22, edgecolor="#b91c1c", facecolor="#fee2e2", lw=0.9)
            ax.add_patch(post_rect)
        else:
            pad_fill = "#dcfce7" if is_gnd_pin else "#f1f5f9"
            pad_edge = "#15803d" if is_gnd_pin else "#64748b"
            post_circ = patches.Circle((px, 5.50), 0.11, edgecolor=pad_edge, facecolor=pad_fill, lw=0.9)
            ax.add_patch(post_circ)

        p_color = "#15803d" if is_gnd_pin else ("#b91c1c" if is_sq_pin else "#0f172a")
        plot_line(ax, px, 5.50, marker="s", markersize=3.6, color=p_color)
        draw_text(ax, px, 5.95, str(pin_idx), fontsize=6.8, fontweight="bold", ha="center", va="center", fontfamily="monospace")

    male_indicator = "MALE PCB HEADER (PIN POSTS PROTRUDE UP)" if is_en else "CONECTOR MACHO NA PCB (PINOS P/ CIMA)"
    draw_text(ax, 12.425, 4.38, male_indicator, fontsize=5.6, fontweight="bold", color="#475569", ha="center", fontfamily="monospace")

    f8_sock = patches.Rectangle((13.80, 4.60), 1.35, 2.25, edgecolor="#000000", facecolor="#e2e8f0", linewidth=1.2)
    ax.add_patch(f8_sock)
    draw_text(ax, 14.475, 7.00, "FP50004\n(Flat 8 Vias)", fontsize=6.8, fontweight="bold", ha="center", fontfamily="monospace")
    render_vertical_conductive_vias(ax, [13.95 + i * 0.150 for i in range(8)])

    draw_annotate(ax, "", xy=(14.80, 4.15), xytext=(10.05, 4.15), arrowprops={"arrowstyle": "->", "color": "#000000", "lw": 1.5})
    seq_lbl = "CANONICAL ORDER: PIN 6 (10V FLAT) <---> PIN 1 (8V FLAT)" if is_en else "ORDEM CANÔNICA: PINO 6 (FLAT 10V) <---> PINO 1 (FLAT 8V)"
    draw_text(ax, 12.425, 3.90, seq_lbl, fontsize=6.2, fontweight="bold", ha="center", fontfamily="monospace")

    nt_box = patches.Rectangle((0.85, 0.58), 8.50, 1.17, edgecolor="#000000", facecolor="#f8fafc", linewidth=1.2)
    ax.add_patch(nt_box)
    cad_hdr = "MECHANICAL CONFORMANCE RULES AND BENCH VISUAL IDENTIFICATION:" if is_en else "REGRAS DE CONFORMIDADE MECÂNICA E IDENTIFICAÇÃO VISUAL DE BANCADA:"
    draw_text(ax, 0.98, 1.62, cad_hdr, fontsize=7.0, fontweight="bold", fontfamily="monospace")

    if is_en:
        cad_notes = (
            "1. THICK vs THIN WALL: Rear wall (0.60mm) faces board interior; thin front wall (0.45mm) houses the locking latch notch.\n"
            "2. HARNESS MATING: Female ZHR-6 connector plugs directly into P7501 header with Wire 1 matching Pin 1, Wire 4 matching GND.\n"
            "3. GROUND PIN 4 (0.0Ω): Pin 4 is GND, verified with direct zero-ohm continuity to metal chassis. Do NOT reverse with Pin 1.\n"
            "4. NSW5V & SW_UART SAFETY: Keep Pin 5 (NSW5V, ~5.15V upstream) Hi-Z ONLY without pulls! Pin 1 (SW_UART) LOW = XFWE serial boot."
        )
    else:
        cad_notes = (
            "1. PAREDE GROSSA vs FINA: A parede traseira (0.60mm) fica para dentro da placa; a parede frontal fina (0.45mm) abriga a trava.\n"
            "2. CONEXÃO DO CHICOTE: O soquete fêmea ZHR-6 conecta diretamente ao conector P7501 com Fio 1 no Pino 1 e Fio 4 no GND.\n"
            "3. TERRA CANÔNICO PINO 4 (0.0Ω): O pino 4 é o GND canônico, verificado com continuidade 0.0Ω ao chassi metálico.\n"
            "4. SEGURANÇA NSW5V/SW_UART: Mantenha Pino 5 (NSW5V ~5.15V upstream) em Hi-Z SEM pulls! Pino 1 LOW força XFWE serial boot."
        )
    draw_text(ax, 0.98, 1.48, cad_notes, fontsize=6.2, color="#334155", ha="left", va="top", fontfamily="sans-serif", linespacing=1.22)

    finalize_and_save_drawing(fig, ax, output_path)


def generate_all_assets() -> None:
    """Generates Canonical English and Portuguese CAD/schematic assets without duplicating shared photos."""
    for d in ("docs/pt-BR/images", "docs/en-US/images"):
        os.makedirs(d, exist_ok=True)

    generate_electrical_schematic("docs/en-US/images/p7501_pico_probe_assembly_schematic.png", lang="en")
    generate_mechanical_cad_drawing("docs/en-US/images/panasonic_dmr_eh55_pcb_p7501_orientation.png", lang="en")
    generate_electrical_schematic("docs/pt-BR/images/p7501_pico_probe_assembly_schematic.png", lang="pt")
    generate_mechanical_cad_drawing("docs/pt-BR/images/panasonic_dmr_eh55_pcb_p7501_orientation.png", lang="pt")

    print("[OK] Generated CAD & schematic assets cleanly in docs/en-US/images/ and docs/pt-BR/images/")


if __name__ == "__main__":
    generate_all_assets()
