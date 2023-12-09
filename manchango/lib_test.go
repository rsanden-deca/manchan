package manchan

import (
	"fmt"
	"reflect"
	"sort"
	"testing"
	"time"
)

func TestChannelPingPong(t *testing.T) {
	tx, rx := NewChannel[string]()
	tx.Send("hello")
	tx.Send("world")
	if msg, _ := rx.Recv(); msg != "hello" { t.Fail() }
	if msg, _ := rx.Recv(); msg != "world" { t.Fail() }
}

func TestChannelIterator(t *testing.T) {
	tx, rx := NewChannel[int]()

	for i := 0; i < 5; i++ {
		tx.Send(i)
	}
	tx.Close()

	msg, ok := rx.Recv(); if !ok { t.FailNow() }
	for i := 0; ok; i++ {
		if i > 4 || msg != i {
			t.FailNow()
		}
		msg, ok = rx.Recv()
	}
}

func TestChannelConcurrent(t *testing.T) {
	tx, rx := NewChannel[string]()
	go func() {
		for i := 0; i < 5; i++ {
			tx.Send(fmt.Sprintf("hello %d", i))
		}
	}()
	for i := 0; i < 5; i++ {
		msg, ok := rx.Recv(); if !ok { t.FailNow() }
		if msg != fmt.Sprintf("hello %d", i) { t.FailNow() }
	}
}

func TestChannelMpsc(t *testing.T) {
	tx, rx := NewChannel[string]()
	tx1 := tx.Clone()
	tx2 := tx.Clone()
	tx3 := tx.Clone()
	tx.Close()

	go func() {
		for i := 0; i < 5; i++ {
			tx1.Send(fmt.Sprintf("hello %d from %d", i, 1))
		}
		tx1.Close()
	}()
	go func() {
		for i := 0; i < 5; i++ {
			tx2.Send(fmt.Sprintf("hello %d from %d", i, 2))
		}
		tx2.Close()
	}()
	go func() {
		for i := 0; i < 5; i++ {
			tx3.Send(fmt.Sprintf("hello %d from %d", i, 3))
		}
		tx3.Close()
	}()

	for i := 0; i < 15; i++ {
		_, ok := rx.Recv(); if !ok { t.FailNow() }
	}
	_, ok := rx.Recv(); if ok { t.FailNow() }
}

func TestChannelSpmc(t *testing.T) {
	tx, rx := NewChannel[string]()
	rx1 := rx.Clone()
	rx2 := rx.Clone()
	rx3 := rx.Clone()

	rx1ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx1.Recv()
			rx1ResultsChan <- msg
			time.Sleep(10000000 * time.Nanosecond)
		}
		close(rx1ResultsChan)
	}()

	rx2ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx2.Recv()
			rx2ResultsChan <- msg
			time.Sleep(20000000 * time.Nanosecond)
		}
		close(rx2ResultsChan)
	}()

	rx3ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx3.Recv()
			rx3ResultsChan <- msg
			time.Sleep(30000000 * time.Nanosecond)
		}
		close(rx3ResultsChan)
	}()

	for i := 0; i < 15; i++ {
		tx.Send(fmt.Sprintf("hello #%02d", i))
	}
	tx.Close()

	results := []string{}
	for msg := range rx1ResultsChan { results = append(results, msg) }
	for msg := range rx2ResultsChan { results = append(results, msg) }
	for msg := range rx3ResultsChan { results = append(results, msg) }
	sort.Strings(results)

	if len(results) != 15 { t.FailNow() }
	if !reflect.DeepEqual(
		results,
		[]string{
			"hello #00",
			"hello #01",
			"hello #02",
			"hello #03",
			"hello #04",
			"hello #05",
			"hello #06",
			"hello #07",
			"hello #08",
			"hello #09",
			"hello #10",
			"hello #11",
			"hello #12",
			"hello #13",
			"hello #14",
		}) { t.FailNow() }
}

func TestChannelMpmc(t *testing.T) {
	tx, rx := NewChannel[string]()
	tx1 := tx.Clone()
	tx2 := tx.Clone()
	tx3 := tx.Clone()
	tx.Close()

	rx1 := rx.Clone()
	rx2 := rx.Clone()
	rx3 := rx.Clone()

	rx1ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx1.Recv()
			rx1ResultsChan <- msg
			time.Sleep(10000000 * time.Nanosecond)
		}
		close(rx1ResultsChan)
	}()

	rx2ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx2.Recv()
			rx2ResultsChan <- msg
			time.Sleep(20000000 * time.Nanosecond)
		}
		close(rx2ResultsChan)
	}()

	rx3ResultsChan := make(chan string)
	go func() {
		for i := 0; i < 5; i++ {
			msg, _ := rx3.Recv()
			rx3ResultsChan <- msg
			time.Sleep(30000000 * time.Nanosecond)
		}
		close(rx3ResultsChan)
	}()

	go func() {
		for i := 0; i < 5; i++ {
			tx1.Send(fmt.Sprintf("hello #%d from tx1", i))
			time.Sleep(11000000 * time.Nanosecond)
		}
		tx1.Close()
	}()
	go func() {
		for i := 0; i < 5; i++ {
			tx2.Send(fmt.Sprintf("hello #%d from tx2", i))
			time.Sleep(13000000 * time.Nanosecond)
		}
		tx2.Close()
	}()
	go func() {
		for i := 0; i < 5; i++ {
			tx3.Send(fmt.Sprintf("hello #%d from tx3", i))
			time.Sleep(15000000 * time.Nanosecond)
		}
		tx3.Close()
	}()

	results := []string{}
	for msg := range rx1ResultsChan { results = append(results, msg) }
	for msg := range rx2ResultsChan { results = append(results, msg) }
	for msg := range rx3ResultsChan { results = append(results, msg) }
	sort.Strings(results)

	if len(results) != 15 { t.FailNow() }
	if !reflect.DeepEqual(
		results,
		[]string{
			"hello #0 from tx1",
			"hello #0 from tx2",
			"hello #0 from tx3",
			"hello #1 from tx1",
			"hello #1 from tx2",
			"hello #1 from tx3",
			"hello #2 from tx1",
			"hello #2 from tx2",
			"hello #2 from tx3",
			"hello #3 from tx1",
			"hello #3 from tx2",
			"hello #3 from tx3",
			"hello #4 from tx1",
			"hello #4 from tx2",
			"hello #4 from tx3",
		}) { t.FailNow() }
	_, ok := rx.Recv(); if ok { t.FailNow() }
	_, ok = rx1.Recv(); if ok { t.FailNow() }
	_, ok = rx2.Recv(); if ok { t.FailNow() }
	_, ok = rx3.Recv(); if ok { t.FailNow() }
}
