module ui

fn test_heights_lookups() {
	mut hs := Heights{}
	hs.resize(1000, 20)
	assert hs.total() == 20000
	assert hs.prefix(10) == 200
	assert hs.find(0) == 0
	assert hs.find(199.5) == 9
	assert hs.find(200) == 10
	assert hs.find(1e9) == 999
	hs.set(5, 100)
	assert hs.prefix(6) == 200
	assert hs.prefix(10) == 280
	assert hs.find(150) == 5
	assert hs.find(200) == 6
	hs.resize(1003, 30) // known heights stay, new rows get the estimate
	assert hs.height(5) == 100
	assert hs.total() == 20000 + 80 + 90
	hs.resize(3, 20)
	assert hs.total() == 60
	assert hs.find(50) == 2
	assert hs.find(130) == 2 // past the end: the last row
}
