use super::*;
use crate::fake::{FakeDevice, info};
use can_usb_protocol::{
    Bitrate, BusError, FirmwareVersion, MAX_PACKET_SIZE, Mode, PayloadError, unpack,
};
use std::pin::Pin;

/// Polls `future` once. The fake finishes every transfer it can at once,
/// so whatever can finish does so on the first poll.
fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

/// The output of `future`, which must not have to wait.
fn now<F: Future>(future: F) -> F::Output {
    match poll(pin!(future)) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("waits"),
    }
}

fn open(fake: &FakeDevice) -> UsbCanDevice<FakeDevice> {
    UsbCanDevice::open_transport(fake.clone(), Config::DEFAULT).unwrap()
}

fn events() -> [BusEvent; 3] {
    [
        BusEvent::Frame {
            timestamp_us: 1,
            frame: Frame::new(0x123u16, &[1, 2, 3]).unwrap(),
        },
        BusEvent::BusError {
            timestamp_us: 2,
            error: BusError::Crc,
        },
        BusEvent::Overrun {
            timestamp_us: 3,
            lost: 4,
        },
    ]
}

fn event(timestamp_us: u32) -> BusEvent {
    BusEvent::Overrun {
        timestamp_us,
        lost: 1,
    }
}

fn packet(events: &[BusEvent]) -> Vec<u8> {
    let mut packer = Packer::new();

    for event in events {
        packer.push(event).unwrap();
    }

    packer.as_bytes().to_vec()
}

/// A standard frame with eight bytes, 12 bytes on the wire, so five fit in
/// a packet.
fn frame(n: u8) -> Frame {
    Frame::new(0x7cfu16, &[n; 8]).unwrap()
}

const FRAMES_PER_PACKET: usize = 5;

/// Every frame the device took, in order.
fn received_frames(fake: &FakeDevice) -> Vec<Frame> {
    fake.received()
        .iter()
        .flat_map(|packet| unpack::<Frame>(packet))
        .collect::<std::result::Result<_, _>>()
        .unwrap()
}

#[test]
fn open_checks_the_version_then_starts_the_bus_at_the_default_config() {
    let fake = FakeDevice::new();
    open(&fake);

    let (payload, len) = Config::DEFAULT.encode();
    assert_eq!(
        fake.controls(),
        [
            (ControlRequest::GetDeviceInfo, vec![]),
            (ControlRequest::SetConfig, payload[..len].to_vec()),
            (ControlRequest::StartBus, vec![]),
        ]
    );
    assert_eq!(fake.reads_queued(), IN_TRANSFERS);
}

#[test]
fn open_with_another_config_sets_that_one() {
    let fake = FakeDevice::new();
    let config = Config {
        bitrate: Bitrate::Kbit250,
        mode: Mode::ListenOnly,
    };
    UsbCanDevice::open_transport(fake.clone(), config).unwrap();

    assert_eq!(fake.controls()[1], (ControlRequest::SetConfig, vec![1, 1]));
    assert_eq!(fake.controls()[2].0, ControlRequest::StartBus);
}

#[test]
fn open_refuses_another_protocol_version_before_sending_anything_else() {
    let fake = FakeDevice::with_info(info(PROTOCOL_VERSION + 1));

    let result = UsbCanDevice::open_transport(fake.clone(), Config::DEFAULT);

    assert!(matches!(
        result,
        Err(Error::ProtocolMismatch { device, host })
            if device == PROTOCOL_VERSION + 1 && host == PROTOCOL_VERSION
    ));
    assert_eq!(fake.controls(), [(ControlRequest::GetDeviceInfo, vec![])]);
    assert_eq!(fake.reads_queued(), 0);
}

#[test]
fn open_fails_when_the_device_refuses_to_start() {
    let fake = FakeDevice::new();
    fake.stall(ControlRequest::StartBus);

    assert!(matches!(
        UsbCanDevice::open_transport(fake, Config::DEFAULT),
        Err(Error::Transfer(TransferError::Stall))
    ));
}

#[test]
fn every_event_in_a_packet_is_received_in_order() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(packet(&events()));
    fake.send_packet(packet(&[event(10)]));

    for expected in events().into_iter().chain([event(10)]) {
        assert_eq!(now(device.receive()).unwrap(), expected);
    }
    assert!(device.try_receive().is_none());
    assert_eq!(fake.reads_queued(), IN_TRANSFERS);
}

#[test]
fn receive_waits_for_a_packet() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    let mut receive = pin!(device.receive());

    assert!(poll(receive.as_mut()).is_pending());

    fake.send_packet(packet(&[event(1)]));

    assert!(matches!(poll(receive.as_mut()), Poll::Ready(Ok(e)) if e == event(1)));
}

#[test]
fn an_empty_packet_carries_nothing() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(vec![]);
    fake.send_packet(packet(&[event(1)]));

    assert_eq!(now(device.receive()).unwrap(), event(1));
}

#[test]
fn a_corrupt_tail_comes_after_the_events_before_it_and_before_the_next_packet() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    let mut corrupt = packet(&events());
    corrupt.push(0xff);
    fake.send_packet(corrupt);
    fake.send_packet(packet(&[event(10)]));

    for expected in events() {
        assert_eq!(now(device.receive()).unwrap(), expected);
    }
    assert!(matches!(now(device.receive()), Err(Error::Unpack(_))));
    assert_eq!(now(device.receive()).unwrap(), event(10));
}

#[test]
fn a_failed_read_is_reported_in_its_place_and_reading_goes_on() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(packet(&[event(1)]));
    fake.fail_read(TransferError::Fault);
    fake.send_packet(packet(&[event(2)]));

    assert_eq!(now(device.receive()).unwrap(), event(1));
    assert!(matches!(
        now(device.receive()),
        Err(Error::Transfer(TransferError::Fault))
    ));
    assert_eq!(now(device.receive()).unwrap(), event(2));
    assert_eq!(fake.reads_queued(), IN_TRANSFERS);
}

#[test]
fn sent_frames_share_a_packet_until_flushed() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    for n in 0..3 {
        now(device.send(&frame(n))).unwrap();
    }
    assert!(fake.received().is_empty());

    now(device.flush()).unwrap();

    assert_eq!(fake.received().len(), 1);
    assert_eq!(received_frames(&fake), [frame(0), frame(1), frame(2)]);
}

#[test]
fn a_full_packet_goes_out_when_the_next_frame_does_not_fit() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    let frames: Vec<Frame> = (0..=FRAMES_PER_PACKET as u8).map(frame).collect();

    for frame in &frames {
        now(device.send(frame)).unwrap();
    }

    assert_eq!(fake.received().len(), 1);
    assert_eq!(received_frames(&fake), frames[..FRAMES_PER_PACKET]);

    now(device.flush()).unwrap();

    assert_eq!(received_frames(&fake), frames);
}

#[test]
fn send_all_puts_the_last_packet_on_its_way() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    let frames: Vec<Frame> = (0..12).map(frame).collect();

    now(device.send_all(&frames)).unwrap();

    assert_eq!(fake.received().len(), 3);
    assert_eq!(received_frames(&fake), frames);
    assert!(fake.received().iter().all(|p| p.len() <= MAX_PACKET_SIZE));
}

#[test]
fn receive_sends_the_packet_being_filled() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    now(device.send(&frame(1))).unwrap();
    let mut receive = pin!(device.receive());
    assert!(poll(receive.as_mut()).is_pending());

    assert_eq!(received_frames(&fake), [frame(1)]);
}

#[test]
fn a_device_that_naks_holds_sends_at_the_transfer_limit() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);

    let fit = OUT_TRANSFERS * FRAMES_PER_PACKET + FRAMES_PER_PACKET;
    for n in 0..fit {
        now(device.send(&frame(n as u8))).unwrap();
    }
    assert_eq!(fake.writes_queued(), OUT_TRANSFERS);

    let blocked = frame(0xee);
    {
        let mut send = pin!(device.send(&blocked));
        assert!(poll(send.as_mut()).is_pending());
        assert!(poll(send.as_mut()).is_pending());
        // Dropped here, as by a timeout.
    }
    assert_eq!(fake.writes_queued(), OUT_TRANSFERS);

    {
        let mut send = pin!(device.send(&blocked));
        assert!(poll(send.as_mut()).is_pending());
        fake.accept(true);
        assert!(matches!(poll(send.as_mut()), Poll::Ready(Ok(()))));
    }
    now(device.flush()).unwrap();

    let mut expected: Vec<Frame> = (0..fit).map(|n| frame(n as u8)).collect();
    expected.push(blocked);
    assert_eq!(received_frames(&fake), expected);
}

#[test]
fn flush_waits_until_the_device_takes_every_packet() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);

    now(device.send(&frame(1))).unwrap();
    let mut flush = pin!(device.flush());

    assert!(poll(flush.as_mut()).is_pending());
    fake.accept(true);
    assert!(matches!(poll(flush.as_mut()), Poll::Ready(Ok(()))));
}

#[test]
fn a_failed_write_is_returned_by_the_next_call_with_its_frames() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.fail_next_write(TransferError::Stall);

    for n in 0..3 {
        now(device.send(&frame(n))).unwrap();
    }

    assert!(matches!(
        now(device.flush()),
        Err(Error::SendFailed {
            frames: 3,
            source: TransferError::Stall
        })
    ));

    now(device.send(&frame(9))).unwrap();
    now(device.flush()).unwrap();
    assert_eq!(received_frames(&fake), [frame(9)]);
}

#[test]
fn a_failed_write_refuses_the_next_frame_and_counts_every_lost_one() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);

    for n in 0..2 * FRAMES_PER_PACKET + 1 {
        now(device.send(&frame(n as u8))).unwrap();
    }
    // The first packet fails, the second goes through.
    fake.fail_next_write(TransferError::Fault);
    fake.accept(true);

    // Taken: the failure was not yet known when it was called.
    now(device.send(&frame(0x20))).unwrap();
    assert!(matches!(
        now(device.send(&frame(0x21))),
        Err(Error::SendFailed {
            frames: FRAMES_PER_PACKET,
            source: TransferError::Fault
        })
    ));
    now(device.flush()).unwrap();

    let mut expected: Vec<Frame> = (FRAMES_PER_PACKET..2 * FRAMES_PER_PACKET + 1)
        .map(|n| frame(n as u8))
        .collect();
    expected.push(frame(0x20));
    assert_eq!(received_frames(&fake), expected);
}

#[test]
fn unplugging_reports_one_disconnect_after_the_events_already_received() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(packet(&events()));
    now(device.send(&frame(1))).unwrap();
    assert!(device.try_receive().is_some());
    fake.disconnect();

    let rest: Vec<_> = std::iter::from_fn(|| device.try_receive())
        .take(10)
        .collect();

    assert_eq!(rest.len(), 10, "keeps reporting");
    assert_eq!(*rest[0].as_ref().unwrap(), events()[1]);
    assert_eq!(*rest[1].as_ref().unwrap(), events()[2]);
    assert!(
        rest[2..]
            .iter()
            .all(|r| matches!(r, Err(Error::Disconnected)))
    );

    assert!(matches!(
        now(device.send(&frame(2))),
        Err(Error::Disconnected)
    ));
    assert!(matches!(now(device.flush()), Err(Error::Disconnected)));
    assert!(matches!(
        now(device.set_config(Config::DEFAULT)),
        Err(Error::Disconnected)
    ));
    assert!(matches!(now(device.stop()), Err(Error::Disconnected)));
    assert_eq!(fake.reads_queued(), 0, "every read came back");
}

#[test]
fn unplugging_ends_a_send_waiting_for_room() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);

    for n in 0..(OUT_TRANSFERS + 1) * FRAMES_PER_PACKET {
        now(device.send(&frame(n as u8))).unwrap();
    }
    let blocked = frame(0xee);
    let mut send = pin!(device.send(&blocked));
    assert!(poll(send.as_mut()).is_pending());

    fake.disconnect();

    assert!(matches!(
        poll(send.as_mut()),
        Poll::Ready(Err(Error::Disconnected))
    ));
}

#[test]
fn set_config_discards_events_from_before_it() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(packet(&events()));
    assert_eq!(now(device.receive()).unwrap(), events()[0]);
    // Already read by a queued transfer, not yet collected.
    fake.send_packet(packet(&[event(10)]));

    now(device.set_config(Config::DEFAULT)).unwrap();
    assert!(device.try_receive().is_none());
    assert_eq!(fake.reads_queued(), IN_TRANSFERS);

    fake.send_packet(packet(&[event(20)]));
    assert_eq!(now(device.receive()).unwrap(), event(20));
}

#[test]
fn set_config_ends_every_write_before_the_request() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);

    let frames: Vec<Frame> = (0..3 * FRAMES_PER_PACKET as u8 + 1).map(frame).collect();
    now(device.send_all(&frames)).unwrap();
    assert!(fake.writes_queued() > 0);

    let config = Config {
        bitrate: Bitrate::Kbit125,
        mode: Mode::Normal,
    };
    now(device.set_config(config)).unwrap();

    assert_eq!(fake.controls().last().unwrap().0, ControlRequest::SetConfig);
    assert_eq!(fake.writes_pending_at_controls().last(), Some(&0));
    assert_eq!(fake.writes_queued(), 0);

    // Nothing from before goes out under the new config, and no error for
    // the discarded frames is left over.
    fake.accept(true);
    now(device.send(&frame(0x30))).unwrap();
    now(device.flush()).unwrap();
    assert_eq!(received_frames(&fake), [frame(0x30)]);
}

#[test]
fn start_discards_like_set_config() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);

    fake.send_packet(packet(&events()));
    now(device.send(&frame(1))).unwrap();
    now(device.start()).unwrap();

    assert_eq!(fake.controls().last().unwrap().0, ControlRequest::StartBus);
    assert!(device.try_receive().is_none());
    now(device.flush()).unwrap();
    assert!(fake.received().is_empty());
}

#[test]
fn a_refused_config_still_leaves_the_reads_queued() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.stall(ControlRequest::SetConfig);

    assert!(matches!(
        now(device.set_config(Config::DEFAULT)),
        Err(Error::Transfer(TransferError::Stall))
    ));
    assert_eq!(fake.reads_queued(), IN_TRANSFERS);

    fake.send_packet(packet(&[event(1)]));
    assert_eq!(now(device.receive()).unwrap(), event(1));
}

#[test]
fn stop_does_not_wait_for_writes() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    fake.accept(false);
    now(device.send_all(&[frame(1)])).unwrap();

    now(device.stop()).unwrap();

    assert_eq!(fake.controls().last().unwrap().0, ControlRequest::StopBus);
}

#[test]
fn a_program_that_only_sends_still_collects_events_up_to_the_limit() {
    let fake = FakeDevice::new();
    let mut device = open(&fake);
    let per_packet = events().len();
    let packets = EVENT_QUEUE_LIMIT / per_packet + 2 * IN_TRANSFERS;

    for _ in 0..packets {
        fake.send_packet(packet(&events()));
    }
    for n in 0..10 {
        now(device.send(&frame(n))).unwrap();
    }

    // The library stops collecting at the limit; the rest waits on the
    // device, which would report what it loses as an overrun.
    assert!(device.events.len() >= EVENT_QUEUE_LIMIT);
    assert!(device.events.len() < EVENT_QUEUE_LIMIT + per_packet);
    assert!(fake.unsent_packets() > 0);

    let mut received = 0;
    while let Some(event) = device.try_receive() {
        assert_eq!(event.unwrap(), events()[received % per_packet]);
        received += 1;
    }
    assert_eq!(received, packets * per_packet, "nothing lost");
}

#[test]
fn device_info_of_this_version_decodes() {
    let fake = FakeDevice::new();
    let device = open(&fake);

    assert_eq!(
        now(device.device_info()).unwrap(),
        DeviceInfo {
            protocol_version: PROTOCOL_VERSION,
            firmware_version: FirmwareVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
            serial: [0xab; DeviceInfo::SERIAL_LEN],
        }
    );
}

/// A newer device may lay out the rest differently; the version alone
/// decides.
#[test]
fn a_mismatch_is_reported_whatever_follows_the_version() {
    let mut bytes = vec![(PROTOCOL_VERSION + 1) as u8];
    bytes.extend_from_slice(&[0xff; 40]);

    assert!(matches!(
        decode_device_info(&bytes),
        Err(Error::ProtocolMismatch { .. })
    ));
}

#[test]
fn a_truncated_device_info_is_a_payload_error() {
    let bytes = info(PROTOCOL_VERSION);

    assert!(matches!(
        decode_device_info(&bytes[..bytes.len() - 1]),
        Err(Error::Payload(PayloadError::Malformed))
    ));
    assert!(matches!(
        decode_device_info(&[]),
        Err(Error::Payload(PayloadError::Malformed))
    ));
}
