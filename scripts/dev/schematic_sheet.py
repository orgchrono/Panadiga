"""IEEE Std 315 / IEC 60617 electrical wiring schematic sheet for Pico <-> Panasonic P7501."""

from __future__ import annotations

import matplotlib.patches as patches

from scripts.dev.cad_primitives import (
    create_drawing_canvas,
    draw_chassis_ground,
    draw_ieee_resistor,
    draw_signal_ground,
    draw_text,
    finalize_and_save_drawing,
    plot_line,
    setup_title_block,
)


def generate_electrical_schematic(output_path: str, lang: str = "en") -> None:
    """Generates standard IEEE Std 315 / IEC 60617 electrical wiring schematic."""
    fig, ax = create_drawing_canvas()

    is_en = lang == "en"
    header_org = (
        "PANADIGA BENCH LAB — RIGOROUS IEEE SCHEMATIC"
        if is_en
        else "PANADIGA BENCH LAB — ESQUEMÁTICO IEEE RIGOROSO"
    )
    header_sub = "IEEE Std 315 / IEC 60617"
    title_text = (
        "TITLE: Pico RP2040 to Panasonic P7501 Probe"
        if is_en
        else "TÍTULO: Sonda Pico RP2040 para Panasonic P7501"
    )
    app_text = "TARGET: VEP79138 (DMR-EH55) / AM34 (MN103S)"
    rev_text = "REV: 3.0 (Verified P7501 115200 8N1 Pinout 1..6)"
    date_scale = "DATE: 2026-11-12 | SCALE: NTS"

    main_hdr = (
        "BENCH ELECTRICAL INTERFACE SCHEMATIC (P7501 ASYNC UART 115200 8N1)"
        if is_en
        else "ESQUEMÁTICO ELÉTRICO DE INTERFACE DE BANCADA (P7501 UART ASSÍNCRONA 115200 8N1)"
    )
    sub_hdr = (
        "Homologated Interface: Raspberry Pi Pico (RP2040) <---> Panasonic Port P7501 [B6B-ZR-3R4]"
        if is_en
        else "Conexão Homologada: Raspberry Pi Pico (RP2040) <---> Conector Panasonic P7501 [B6B-ZR-3R4]"
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

    rp_box = patches.Rectangle((0.85, 2.05), 3.25, 6.20, edgecolor="#0f172a", facecolor="#f1f5f9", linewidth=1.5)
    ax.add_patch(rp_box)
    rp_lbl = (
        "BENCH CONTROLLER\nRASPBERRY PI PICO\n(RP2040 3.3V LVTTL)"
        if is_en
        else "CONTROLE DE BANCADA\nRASPBERRY PI PICO\n(RP2040 3.3V LVTTL)"
    )
    draw_text(ax, 2.47, 8.00, rp_lbl, fontsize=8.0, fontweight="bold", ha="center", fontfamily="monospace")

    target_box = patches.Rectangle((11.90, 2.05), 3.45, 6.20, edgecolor="#0f172a", facecolor="#f1f5f9", linewidth=1.5)
    ax.add_patch(target_box)
    tgt_lbl = (
        "DIGITAL BOARD VEP79138\nCONNECTOR P7501\n(JST ZR 1.5mm B6B-ZR)"
        if is_en
        else "PLACA DIGITAL VEP79138\nCONECTOR P7501\n(JST ZR 1.5mm B6B-ZR)"
    )
    draw_text(ax, 13.62, 8.00, tgt_lbl, fontsize=8.0, fontweight="bold", ha="center", fontfamily="monospace")

    opt_tag = "[OPTIONAL / DIRECT WIRE]" if is_en else "[OPCIONAL / FIO DIRETO]"
    nets = [
        (
            7.40,
            "Pin 31 (ADC0)" if is_en else "Pino 31 (ADC0)",
            "GP26",
            "R5",
            "1.0 kΩ",
            "[OPTIONAL / ADC FILTER]" if is_en else "[OPCIONAL / FILTRO ADC]",
            False,
            "Pin 1: SW_UART" if is_en else "Pino 1: SW_UART",
            "XFWE Serial Boot / ADC Sense (~3.25V Hi-Z)",
            False,
            False,
        ),
        (
            6.40,
            "Pin 7 (GPIO)" if is_en else "Pino 7 (GPIO)",
            "GP5",
            "R1",
            "100 Ω",
            "",
            False,
            "Pin 2: XRESET" if is_en else "Pino 2: XRESET",
            "Target Reset (Open-Drain, 0V)",
            False,
            True,
        ),
        (
            5.40,
            "Pin 6 (UART0 TX)" if is_en else "Pino 6 (UART0 TX)",
            "GP4",
            "R2",
            "100 Ω",
            opt_tag,
            False,
            "Pin 3: UARTM2P_T" if is_en else "Pino 3: UARTM2P_T",
            "Pico TX -> AM34 RX via Q7514 (115200 8N1)",
            False,
            False,
        ),
        (
            4.40,
            "Pin 38 (GND)" if is_en else "Pino 38 (GND)",
            "GND COMMON" if is_en else "GND COMUM",
            None,
            None,
            None,
            False,
            "Pin 4: GND" if is_en else "Pino 4: GND",
            "Chassis Ground (0.0Ω Bond)" if is_en else "Terra Chassi (0.0Ω Chassi)",
            True,
            False,
        ),
        (
            3.40,
            "Pin 5 (Hi-Z IN)" if is_en else "Pino 5 (Hi-Z IN)",
            "GP3",
            "R3",
            "Hi-Z ONLY",
            "[NO PULLS!]" if is_en else "[SEM PULLS!]",
            True,
            "Pin 5: NSW5V" if is_en else "Pino 5: NSW5V",
            "IC1506 ~5.15V Rail / ~3.29V Idle (NEVER TX!)",
            False,
            False,
        ),
        (
            2.40,
            "Pin 4 (UART0 RX)" if is_en else "Pino 4 (UART0 RX)",
            "GP2",
            "R4",
            "100 Ω",
            opt_tag,
            False,
            "Pin 6: UARTP2M_T" if is_en else "Pino 6: UARTP2M_T",
            "AM34 TX -> Pico RX (115200 8N1)",
            False,
            False,
        ),
    ]

    for y, p_pin, p_lbl, r_name, r_val, r_opt, has_pu, t_pin, t_lbl, is_gnd, is_od in nets:
        plot_line(ax, [4.10, 4.70], [y, y], color="#000000", linewidth=1.4)
        plot_line(ax, 4.10, y, marker="o", markersize=3.5, color="#000000")
        draw_text(ax, 3.95, y, f"{p_pin}\n{p_lbl}", fontsize=7.0, fontweight="bold", ha="right", va="center", fontfamily="monospace")

        plot_line(ax, [11.30, 11.90], [y, y], color="#000000", linewidth=1.4)
        plot_line(ax, 11.90, y, marker="o", markersize=3.5, color="#000000")
        draw_text(ax, 12.05, y, f"{t_pin}\n{t_lbl}", fontsize=7.0, fontweight="bold", ha="left", va="center", fontfamily="monospace")

        if is_gnd:
            plot_line(ax, [4.70, 11.30], [y, y], color="#000000", linewidth=1.8)
            draw_signal_ground(ax, 5.10, y, color="#000000", lw=1.2)
            draw_chassis_ground(ax, 10.90, y, color="#000000", lw=1.4)
            gnd_msg = (
                "EQUIPOTENTIAL COMMON GROUND BUS (0.0Ω TO CHASSIS ENCLOSURE)"
                if is_en
                else "MALHA DE TERRA EQUIPOTENCIAL COMUM (0.0Ω COM A LATA DO CHASSI)"
            )
            can_msg = (
                "CANONICAL MANUAL PIN 4: GROUND CONTINUITY VERIFICATION TARGET"
                if is_en
                else "MANUAL CANÔNICO: PINO 4 = TERRA (PONTO DE TESTE CONTINUIDADE 0.0Ω)"
            )
            draw_text(ax, 8.00, y + 0.22, gnd_msg, fontsize=7.0, fontweight="bold", color="#15803d", ha="center", fontfamily="monospace")
            draw_text(ax, 8.00, y - 0.22, can_msg, fontsize=7.2, fontweight="bold", color="#b91c1c", ha="center", fontfamily="monospace")
        else:
            plot_line(ax, [4.70, 6.20], [y, y], color="#000000", linewidth=1.4)
            draw_ieee_resistor(
                ax,
                6.20,
                y,
                7.60,
                y,
                label=r_name or "",
                value=r_val or "",
                opt_text=r_opt or "",
                color="#000000",
                lw=1.3,
            )
            plot_line(ax, [7.60, 11.30], [y, y], color="#000000", linewidth=1.4)

            if has_pu:
                hiz_msg = (
                    "STRICT Hi-Z INPUT WITHOUT PULLS: IC1506/D7510 ~5.15V rail (~3.29V idle); NEVER drive push-pull!"
                    if is_en
                    else "ENTRADA Hi-Z ESTRITA SEM PULLS: Linha IC1506/D7510 ~5.15V (~3.29V repouso); NUNCA forçar push-pull!"
                )
                draw_text(ax, 8.00, y - 0.32, hiz_msg, fontsize=5.8, fontweight="bold", color="#b91c1c", ha="center", fontfamily="monospace")

            if is_od:
                od_msg = (
                    "OPEN-DRAIN MODE: GP5 STRICTLY SINKS TO 0V; RELEASES IN Hi-Z\n"
                    "NEVER DRIVE +3.3V PUSH-PULL (SHORT CIRCUIT HAZARD WITH BOARD RESET SUPERVISOR)"
                    if is_en
                    else "MODO OPEN-DRAIN: GP5 DRENA EXCLUSIVAMENTE PARA 0V; LIBERA EM Hi-Z\n"
                    "JAMAIS FORÇAR +3.3V PUSH-PULL (RISCO DE CURTO COM CIRCUITO DE RESET DA PLACA)"
                )
                draw_text(ax, 6.90, y - 0.36, od_msg, fontsize=5.8, fontweight="bold", color="#c2410c", ha="center", fontfamily="monospace", linespacing=1.15)

    nt_box = patches.Rectangle((0.85, 0.58), 8.50, 1.17, edgecolor="#b91c1c", facecolor="#fef2f2", linewidth=1.2)
    ax.add_patch(nt_box)
    nt_hdr = (
        "BENCH ENGINEERING NOTES & WIRING DIRECTIVE (RESISTORS vs DIRECT WIRE):"
        if is_en
        else "NOTAS DE ENGENHARIA DE BANCADA E DIRETRIZ DE LIGAÇÃO (RESISTORES vs FIO DIRETO):"
    )
    draw_text(ax, 0.98, 1.62, nt_hdr, fontsize=7.0, fontweight="bold", color="#991b1b", fontfamily="monospace")

    if is_en:
        notes_str = (
            "• P7501 ASYNC UART (115200 8N1): R7570/R7572 (10kΩ) on Main PCB already bias UART lines. Power Pico ONLY via USB/VSYS.\n"
            "• SW_UART / Q7514 INTERLOCK: Pulling Pin 1 (SW_UART) LOW sets XFWE=LOW and cuts off Q7514 (Pin 3 UARTM2P_T). Use Hi-Z for TX.\n"
            "• VERIFIED PINOUT: Pin 1=SW_UART, Pin 2=XRESET, Pin 3=UARTM2P_T, Pin 4=GND (0.0Ω), Pin 5=NSW5V (Hi-Z), Pin 6=UARTP2M_T."
        )
    else:
        notes_str = (
            "• P7501 UART ASSÍNCRONA (115200 8N1): R7570/R7572 (10kΩ) na Main PCB já polarizam UART. Alimente o Pico SÓ via USB/VSYS.\n"
            "• INTERTRAVA SW_UART / Q7514: Pino 1 LOW força XFWE=LOW e corta Q7514 (Pino 3 UARTM2P_T). Mantenha SW_UART=Hi-Z para TX.\n"
            "• PINAGEM VERIFICADA: 1=SW_UART, 2=XRESET, 3=UARTM2P_T, 4=GND (0.0Ω), 5=NSW5V (Hi-Z), 6=UARTP2M_T."
        )
    draw_text(ax, 0.98, 1.48, notes_str, fontsize=6.2, color="#7f1d1d", ha="left", va="top", fontfamily="sans-serif", linespacing=1.22)

    finalize_and_save_drawing(fig, ax, output_path)
