module fuzz

fn test_same_seed_same_stream() {
	mut a := new_rng(7)
	mut b := new_rng(7)
	corpus := ['hello'.bytes(), '[1;2H'.bytes()]
	for _ in 0 .. 1000 {
		assert a.mutate(corpus[0], corpus) == b.mutate(corpus[0], corpus)
	}
	mut w := watch('self', 100)
	for i in 0 .. 100 {
		w.start()
		_ = a.mutate(corpus[1], corpus)
		w.done(i, 0, 1000)
	}
	w.finish(64)
}
