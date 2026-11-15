"""Strictly typed IEEE Std 315 / IEC 60617 drawing primitives for Panadiga bench schematics."""

from __future__ import annotations

import math
import os
import shutil
import time
from typing import Any, Mapping, Sequence, cast
import matplotlib.patches as patches
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.figure import Figure


def plot_line(
    ax: Axes,
    x: float | Sequence[float],
    y: float | Sequence[float],
    *,
    color: str = "black",
    linewidth: float | None = None,
    solid_capstyle: str | None = None,
    marker: str | None = None,
    markersize: float | None = None,
) -> None:
    """Strictly typed wrapper for Axes.plot keeping Pyright strict happy."""
    kwargs: dict[str, Any] = {"color": color}
    if linewidth is not None:
        kwargs["linewidth"] = linewidth
    if solid_capstyle is not None:
        kwargs["solid_capstyle"] = solid_capstyle
    if marker is not None:
        kwargs["marker"] = marker
    if markersize is not None:
        kwargs["markersize"] = markersize
    cast(Any, ax).plot(x, y, **kwargs)


def draw_text(
    ax: Axes,
    x: float,
    y: float,
    s: str,
    *,
    fontsize: float,
    fontweight: str | None = None,
    color: str = "#000000",
    ha: str = "left",
    va: str = "baseline",
    fontfamily: str = "sans-serif",
    linespacing: float | None = None,
) -> None:
    """Strictly typed wrapper for Axes.text keeping Pyright strict happy."""
    kwargs: dict[str, Any] = {
        "fontsize": fontsize,
        "color": color,
        "ha": ha,
        "va": va,
        "fontfamily": fontfamily,
    }
    if fontweight is not None:
        kwargs["fontweight"] = fontweight
    if linespacing is not None:
        kwargs["linespacing"] = linespacing
    cast(Any, ax).text(x, y, s, **kwargs)


def draw_annotate(
    ax: Axes,
    s: str,
    *,
    xy: tuple[float, float],
    xytext: tuple[float, float],
    arrowprops: Mapping[str, Any],
) -> None:
    """Strictly typed wrapper for Axes.annotate keeping Pyright strict happy."""
    cast(Any, ax).annotate(s, xy=xy, xytext=xytext, arrowprops=dict(arrowprops))


def _savefig(path: str, *, dpi: int, bbox_inches: str, facecolor: Any) -> None:
    """Strictly typed wrapper for pyplot.savefig keeping Pyright strict happy."""
    cast(Any, plt).savefig(path, dpi=dpi, bbox_inches=bbox_inches, facecolor=facecolor)


def draw_ieee_resistor(
    ax: Axes,
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    label: str = "",
    value: str = "",
    opt_text: str = "",
    color: str = "black",
    lw: float = 1.3,
) -> None:
    """Draws a standard IEEE zigzag resistor between (x1, y1) and (x2, y2)."""
    dx = x2 - x1
    dy = y2 - y1
    length = math.hypot(dx, dy)
    angle = math.atan2(dy, dx)

    lead = 0.22 * length
    body = 0.56 * length
    n_peaks = 6
    w = 0.15

    pts: list[tuple[float, float]] = [(0.0, 0.0), (lead, 0.0)]
    step = body / n_peaks
    for i in range(n_peaks):
        sign = 1 if i % 2 == 0 else -1
        pts.append((lead + (i + 0.5) * step, sign * w))
    pts.append((lead + body, 0.0))
    pts.append((length, 0.0))

    cos_a = math.cos(angle)
    sin_a = math.sin(angle)
    rot_pts = [(x1 + px * cos_a - py * sin_a, y1 + px * sin_a + py * cos_a) for px, py in pts]

    xs, ys = zip(*rot_pts)
    plot_line(ax, xs, ys, color=color, linewidth=lw, solid_capstyle="round")

    mid_x = (x1 + x2) / 2.0
    mid_y = (y1 + y2) / 2.0
    off_x = -sin_a * 0.24
    off_y = cos_a * 0.24

    if label or value:
        top_lbl = f"{label}: {value}" if (label and value) else (label or value)
        draw_text(
            ax,
            mid_x + off_x,
            mid_y + off_y,
            top_lbl,
            fontsize=6.8,
            fontweight="bold",
            color=color,
            ha="center",
            va="bottom",
            fontfamily="monospace",
        )

    if opt_text:
        draw_text(
            ax,
            mid_x - off_x,
            mid_y - off_y,
            opt_text,
            fontsize=5.8,
            fontweight="bold",
            color="#475569",
            ha="center",
            va="top",
            fontfamily="monospace",
        )


def draw_chassis_ground(ax: Axes, x: float, y: float, color: str = "black", lw: float = 1.4) -> None:
    """Draws standard IEEE chassis ground symbol (3 diagonal lines)."""
    plot_line(ax, [x, x], [y, y - 0.20], color=color, linewidth=lw)
    plot_line(ax, [x - 0.20, x + 0.20], [y - 0.20, y - 0.20], color=color, linewidth=lw)
    for off in [-0.16, 0.0, 0.16]:
        plot_line(ax, [x + off, x + off - 0.10], [y - 0.20, y - 0.35], color=color, linewidth=lw)


def draw_signal_ground(ax: Axes, x: float, y: float, color: str = "black", lw: float = 1.4) -> None:
    """Draws standard IEEE signal ground symbol (triangle)."""
    plot_line(ax, [x, x], [y, y - 0.16], color=color, linewidth=lw)
    tri = patches.Polygon(
        [[x - 0.16, y - 0.16], [x + 0.16, y - 0.16], [x, y - 0.35]],
        closed=True,
        edgecolor=color,
        facecolor="none",
        linewidth=lw,
    )
    ax.add_patch(tri)


def create_drawing_canvas() -> tuple[Figure, Axes]:
    """Instantiates a standardized white drawing sheet for IEEE engineering drawings."""
    fig, ax = plt.subplots(figsize=(16, 10), dpi=300)
    fig.patch.set_facecolor("#ffffff")
    ax.set_facecolor("#ffffff")
    return fig, ax


def setup_title_block(
    ax: Axes,
    header_org: str,
    header_sub: str,
    title_text: str,
    app_text: str,
    rev_text: str,
    date_scale: str,
    sheet_title: str = "",
    sheet_subtitle: str = "",
) -> None:
    """Renders standardized ISO/IEEE border and engineering title block in footer area."""
    plot_line(ax, [0.5, 15.5, 15.5, 0.5, 0.5], [0.45, 0.45, 9.55, 9.55, 0.45], color="#000000", linewidth=2.0)
    plot_line(ax, [0.65, 15.35, 15.35, 0.65, 0.65], [0.58, 0.58, 9.42, 9.42, 0.58], color="#333333", linewidth=0.8)

    tb = patches.Rectangle((9.5, 0.58), 5.85, 1.17, edgecolor="#000000", facecolor="#f8fafc", linewidth=1.2)
    ax.add_patch(tb)
    plot_line(ax, [9.5, 15.35], [1.36, 1.36], color="#000000", linewidth=0.8)
    plot_line(ax, [9.5, 15.35], [0.97, 0.97], color="#000000", linewidth=0.8)
    plot_line(ax, [12.8, 12.8], [0.58, 1.36], color="#000000", linewidth=0.8)

    draw_text(ax, 9.65, 1.54, f"{header_org} | {header_sub}", fontsize=6.8, fontweight="bold", fontfamily="monospace")
    draw_text(ax, 9.65, 1.16, title_text, fontsize=7.2, fontweight="bold", fontfamily="sans-serif")
    draw_text(ax, 9.65, 0.77, app_text, fontsize=6.8, fontfamily="sans-serif")
    draw_text(ax, 12.95, 1.16, rev_text, fontsize=6.8, fontweight="bold", fontfamily="monospace")
    draw_text(ax, 12.95, 0.77, date_scale, fontsize=6.8, fontfamily="monospace")

    if sheet_title:
        draw_text(ax, 8.0, 9.05, sheet_title, fontsize=11.5, fontweight="bold", ha="center", fontfamily="monospace")
    if sheet_subtitle:
        draw_text(ax, 8.0, 8.75, sheet_subtitle, fontsize=8.0, color="#475569", ha="center", fontfamily="sans-serif")


def finalize_and_save_drawing(fig: Figure, ax: Axes, output_path: str) -> None:
    """Closes and persists the vector figure to disk."""
    ax.set_xlim(0, 16)
    ax.set_ylim(0, 10)
    ax.axis("off")
    plt.tight_layout()
    abs_path = os.path.abspath(output_path)
    os.makedirs(os.path.dirname(abs_path), exist_ok=True)
    tmp_path = abs_path + ".tmp.png"
    try:
        _savefig(tmp_path, dpi=300, bbox_inches="tight", facecolor=fig.get_facecolor())
        plt.close(fig)
        for _attempt in range(5):
            try:
                if os.path.exists(abs_path):
                    os.remove(abs_path)
                os.replace(tmp_path, abs_path)
                break
            except OSError:
                time.sleep(0.15)
        else:
            if os.path.exists(tmp_path):
                shutil.move(tmp_path, abs_path)
    finally:
        plt.close("all")
        if os.path.exists(tmp_path):
            try:
                os.remove(tmp_path)
            except OSError:
                pass
    print(f"[OK] Generated {abs_path}")


def render_vertical_conductive_vias(ax: Axes, x_coords: list[float]) -> None:
    """Draws vertical conductive FFC vias with contact pads and ribbon traces."""
    for vx in x_coords:
        plot_line(ax, [vx, vx], [4.80, 6.60], color="#334155", linewidth=1.2)
        plot_line(ax, vx, 4.85, marker="s", markersize=2.2, color="#0f172a")
        plot_line(ax, vx, 6.55, marker=".", markersize=2.0, color="#64748b")
