"""Read-only GDB RSP (`$m` / `$x` / `$g`) memory dumper and bridge stream helpers for AM34."""

from __future__ import annotations

import hashlib
import importlib
import time

from scripts.bench.rsp_frames import (
    DumpExtractionResult as DumpExtractionResult,
    G_REGISTER_BYTES,
    MAX_M_CHUNK,
    MAX_X_CHUNK,
    ParsedRspFrame as ParsedRspFrame,
    RawByteStream as RawByteStream,
    build_rsp_frame as build_rsp_frame,
    decode_mn103s_g_registers as decode_mn103s_g_registers,
    decode_rsp_hex_memory as decode_rsp_hex_memory,
    parse_rsp_reply as parse_rsp_reply,
    raise_if_e0x,
    rsp_checksum as rsp_checksum,
    validate_dump_range as validate_dump_range,
)


class BridgeByteStream:
    """Context-managed Pico `BRIDGE` raw passthrough ending deterministically with `~~~` + `ISOLATE`."""

    def __init__(self, port: str, baud: int = 115200, boot_mode: str = "") -> None:
        serial_mod = importlib.import_module("serial")
        serial_cls = getattr(serial_mod, "Serial")
        self._ser: RawByteStream = serial_cls(port, baudrate=115200, timeout=0.05)
        self._ser.write(b"STATUS\r\n")
        status_bytes = self._drain_any((b"@P7501_RES cmd=STATUS", b"VDD:", b"PANADIGA>"), timeout_s=1.5)
        status_text = status_bytes.decode("utf-8", errors="replace").upper()
        if "NSW5V=0" in status_text or "FAULT_LATCH=1" in status_text:
            try:
                self._ser.write(b"ISOLATE\r\n")
                self._drain_any((b"@P7501_RES cmd=ISOLATE status=OK", b"ISOLATE: OK"), timeout_s=1.0)
            finally:
                self._ser.close()
            raise RuntimeError(f"STANDBY_GATE_STOP: refused BRIDGE entry (STATUS={status_text.strip()})")
        cmd = f"BRIDGE {baud} {boot_mode}".strip() + "\r\n"
        self._ser.write(cmd.encode("ascii"))
        self._drain_any((b"active", b"@P7501_RES cmd=BRIDGE"), timeout_s=2.0)

    def _drain_any(self, markers: tuple[bytes, ...], timeout_s: float) -> bytes:
        buf = bytearray()
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            chunk = self._ser.read(1024)
            if chunk:
                buf.extend(chunk)
                if any(m in buf for m in markers):
                    break
            else:
                time.sleep(0.01)
        return bytes(buf)

    def write(self, data: bytes) -> int | None:
        """Forward raw bytes into active Pico `BRIDGE` session."""
        return self._ser.write(data)

    def read(self, size: int = 1) -> bytes:
        """Read raw UART bytes streamed by P7501 SC0 through `BRIDGE`."""
        return self._ser.read(size)

    def close(self) -> None:
        """Exit `BRIDGE` via 60ms Hayes guard + `~~~`, enforce `ISOLATE`, and close serial device."""
        try:
            time.sleep(0.065)
            self._ser.write(b"~~~")
            self._drain_any(
                (b"@P7501_RES cmd=BRIDGE", b"BRIDGE: OK", b"BRIDGE: ERR"),
                timeout_s=1.5,
            )
            self._ser.write(b"ISOLATE\r\n")
            self._drain_any((b"@P7501_RES cmd=ISOLATE status=OK", b"ISOLATE: OK"), timeout_s=1.0)
        finally:
            self._ser.close()


class MockAm34RspStream:
    """Deterministic AM34 `_dev_SERIAL` simulator (`FUN_574f0590`) for CI and dry-runs."""

    def __init__(
        self,
        base_addr: int,
        memory_image: bytes,
        *,
        boot_noise: bytes = b"DDR CHECK: OK\r\nwait 1 second.\r\n",
        corrupt_chunk_index: int | None = None,
    ) -> None:
        self._base = base_addr
        self._mem = memory_image
        self._rx = bytearray(boot_noise)
        self._corrupt_idx = corrupt_chunk_index
        self._chunk_seen = 0
        self._in_loop = False
        self._closed = False
        self.tx_history: list[bytes] = []

    def write(self, data: bytes) -> int | None:
        """Simulate AM34 RSP frame parsing and reply generation."""
        self.tx_history.append(data)
        if data == b"\x15":
            self._in_loop = True
            self._rx.extend(b"+" + build_rsp_frame(b"OK"))
            return len(data)
        if not self._in_loop:
            return len(data)
        idx = 0
        while idx < len(data):
            if data[idx : idx + 1] in (b"+", b"-", b"\x0e", b"\x04", b"\x02"):
                idx += 1
                continue
            frame = parse_rsp_reply(data[idx:])
            if frame is None:
                break
            self._handle_payload(frame.payload)
            hash_rel = data[idx:].find(b"#")
            idx += (hash_rel + 3) if hash_rel >= 0 else len(data)
        return len(data)

    def _handle_payload(self, payload: bytes) -> None:
        text = payload.decode("ascii", errors="replace")
        if text == "g":
            regs = "00" * G_REGISTER_BYTES
            self._rx.extend(b"+" + build_rsp_frame(regs))
        elif text in ("Z0", "Z1"):
            self._rx.extend(b"+" + build_rsp_frame(b"OK"))
            if text == "Z0":
                self._in_loop = False
        elif text[:1] in ("m", "x") and "," in text:
            addr_s, len_s = text[1:].split(",", 1)
            addr = int(addr_s, 16)
            length = int(len_s, 16)
            off = addr - self._base
            max_chunk = MAX_X_CHUNK if text[0] == "x" else MAX_M_CHUNK
            if off < 0 or length <= 0 or length > max_chunk or (off + length) > len(self._mem):
                self._rx.extend(b"+" + build_rsp_frame(b"E01"))
                return
            self._chunk_seen += 1
            mem_slice = self._mem[off : off + length]
            reply_payload: str | bytes = mem_slice if text[0] == "x" else mem_slice.hex()
            raw_reply = build_rsp_frame(reply_payload)
            if self._corrupt_idx == self._chunk_seen:
                self._corrupt_idx = None
                bad_csum = b"00" if raw_reply.endswith(b"ff") else b"ff"
                raw_reply = raw_reply[:-2] + bad_csum
            self._rx.extend(b"+" + raw_reply)
        else:
            self._rx.extend(b"+" + build_rsp_frame(b"E03"))

    def read(self, size: int = 1) -> bytes:
        """Pop simulated RX bytes."""
        out = bytes(self._rx[:size])
        del self._rx[:size]
        return out

    def close(self) -> None:
        """Mark simulator stream closed."""
        self._closed = True


class ReadOnlyRspDumper:
    """Strictly read-only GDB RSP client extracting full AM34 memory ranges reliably."""

    def __init__(self, stream: RawByteStream, *, reply_timeout_s: float = 1.0) -> None:
        self._stream = stream
        self._timeout_s = reply_timeout_s

    def _read_frame(self, *, expected_payload_len: int | None = None) -> ParsedRspFrame:
        buf = bytearray()
        deadline = time.monotonic() + self._timeout_s
        while time.monotonic() < deadline:
            chunk = self._stream.read(512)
            if chunk:
                buf.extend(chunk)
                parsed = parse_rsp_reply(bytes(buf), expected_payload_len=expected_payload_len)
                if parsed is not None:
                    return parsed
            else:
                time.sleep(0.002)
        raise TimeoutError(f"RSP reply timeout after {len(buf)} RX bytes")

    def enter_stub(self, *, use_magic: bool = False) -> ParsedRspFrame:
        """Drain banner chatter and send optional `0E0402` + `0x15` NAK to enter `_dev_SERIAL` RSP loop."""
        while self._stream.read(1024):
            pass
        if use_magic:
            self._stream.write(bytes((0x0E, 0x04, 0x02)))
            time.sleep(0.02)
            while self._stream.read(1024):
                pass
        self._stream.write(b"\x15")
        return self._read_frame()

    def _exchange_with_retry(self, payload: str, *, max_retries: int) -> ParsedRspFrame:
        req = build_rsp_frame(payload)
        for attempt in range(max_retries + 1):
            self._stream.write(req)
            frame = self._read_frame()
            if frame.checksum_ok:
                self._stream.write(b"+")
                return frame
            self._stream.write(b"-")
            if attempt >= max_retries:
                raise RuntimeError(f"RSP checksum failure on '{payload}' after {max_retries + 1} attempts")
        raise AssertionError("unreachable")

    def read_registers(self, *, max_retries: int = 3) -> dict[str, int]:
        """Query `$g#67` within an active stub session and decode 28 MN103S register words."""
        reply = self._exchange_with_retry("g", max_retries=max_retries)
        return decode_mn103s_g_registers(reply.payload)

    def read_memory(
        self,
        start_addr: int,
        total_bytes: int,
        chunk_size: int = 512,
        *,
        use_magic: bool = False,
        use_binary_x: bool = False,
        exit_stub: bool = True,
        max_retries: int = 3,
    ) -> DumpExtractionResult:
        """Read `[start_addr .. start_addr + total_bytes)` via `$m` or binary `$x` chunks with CRC retries."""
        max_chunk = MAX_X_CHUNK if use_binary_x else MAX_M_CHUNK
        validate_dump_range(start_addr, total_bytes, chunk_size, max_chunk_size=max_chunk)
        self.enter_stub(use_magic=use_magic)

        opcode = "x" if use_binary_x else "m"
        assembled = bytearray()
        chunks_ok = 0
        retries = 0
        exit_ok = False
        try:
            offset = 0
            while offset < total_bytes:
                req_len = min(chunk_size, total_bytes - offset)
                req_payload = f"{opcode}{start_addr + offset:x},{req_len:x}"
                req_bytes = build_rsp_frame(req_payload)
                chunk_bytes: bytes | None = None
                for attempt in range(max_retries + 1):
                    self._stream.write(req_bytes)
                    reply = self._read_frame(expected_payload_len=req_len if use_binary_x else None)
                    if reply.checksum_ok:
                        self._stream.write(b"+")
                        if use_binary_x:
                            if len(reply.payload) != req_len:
                                raise_if_e0x(reply.payload)
                                raise RuntimeError(
                                    f"Unexpected $x payload length at 0x{start_addr + offset:08X}: "
                                    f"expected {req_len}, got {len(reply.payload)}"
                                )
                            chunk_bytes = reply.payload
                        else:
                            chunk_bytes = decode_rsp_hex_memory(reply.payload)
                        break
                    retries += 1
                    self._stream.write(b"-")
                    if attempt >= max_retries:
                        raise RuntimeError(f"Chunk failure at 0x{start_addr + offset:08X}")
                assert chunk_bytes is not None
                assembled.extend(chunk_bytes)
                offset += len(chunk_bytes)
                chunks_ok += 1
            if exit_stub:
                z_reply = self._exchange_with_retry("Z0", max_retries=max_retries)
                exit_ok = z_reply.payload == b"OK"
        finally:
            self._stream.close()

        raw_out = bytes(assembled)
        digest = hashlib.sha256(raw_out).hexdigest()
        return DumpExtractionResult(
            start_addr=start_addr,
            data=raw_out,
            sha256=digest,
            chunks_ok=chunks_ok,
            retries=retries,
            exit_ok=exit_ok,
        )
