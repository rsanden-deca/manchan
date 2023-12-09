package manchan

import "sync"

type Inner[T any] struct {
	sync.Mutex
	queue     []T
	n_senders uint
}

type Shared[T any] struct {
	inner     *Inner[T]
	available *sync.Cond
}

type Sender[T any] struct {
	shared    *Shared[T]
	is_closed bool
}

type Receiver[T any] struct {
	shared *Shared[T]
}

func NewChannel[T any]() (*Sender[T], *Receiver[T]) {
	inner := &Inner[T]{n_senders: 1}
	shared := &Shared[T]{inner: inner, available: sync.NewCond(inner)}
	tx := &Sender[T]{shared: shared, is_closed: false}
	rx := &Receiver[T]{shared: shared}
	return tx, rx
}

func (me *Sender[T]) Clone() *Sender[T] {
	me.shared.inner.Lock()
	defer me.shared.inner.Unlock()
	me.shared.inner.n_senders += 1
	return &Sender[T]{shared: me.shared}
}

func (me *Sender[T]) Close() {
	channel_closed := false
	me.shared.inner.Lock()
	me.is_closed = true
	me.shared.inner.n_senders -= 1
	if me.shared.inner.n_senders == 0 {
		channel_closed = true
	}
	me.shared.inner.Unlock()
	if channel_closed {
		me.shared.available.Broadcast()
	}
}

func (me *Sender[T]) Send(msg T) {
	if me.is_closed {
		panic("Attempt to send on closed sender")
	}
	me.shared.inner.Lock()
	me.shared.inner.queue = append(me.shared.inner.queue, msg)
	me.shared.inner.Unlock()
	me.shared.available.Signal()
}

func (me *Receiver[T]) Clone() *Receiver[T] {
	return &Receiver[T]{shared: me.shared}
}

func (me *Receiver[T]) Recv() (T, bool) {
	me.shared.inner.Lock()
	for {
		if len(me.shared.inner.queue) > 0 {
			msg := me.shared.inner.queue[0]
			me.shared.inner.queue = me.shared.inner.queue[1:]
			me.shared.inner.Unlock()
			return msg, true
		}
		if me.shared.inner.n_senders == 0 {
			me.shared.inner.Unlock()
			return *new(T), false
		}
		me.shared.available.Wait()
	}
}
