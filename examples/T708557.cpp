// Problem: T708557 西安之泪
// Contest: Luogu
// URL: https://www.luogu.com.cn/problem/T708557?contestId=296933
// Memory Limit: 512 MB
// Time Limit: 2000 ms

// Author: gyj2006
/* clang-format off */
#include <bits/stdc++.h>
// #define int long long
#define F(i,l,r) for(int i=l;i<r;++i)
#define Fe(i,l,r) for(int i=l;i<=r;++i)
#define Fer(i,l,r) for(int i=l;i>=r;--i)
typedef __int128 lll;
#define debug(x) cout << #x": " << x <<endl;
#define endl '\n'
#define all(x) (x).begin(),(x).end()
#define rall(x) (x).rbegin(), (x).rend()
#define enr(i, u) for(int i=h[u];i;i=ne[i])
#define fitr(c, itr) for(auto itr=(c).begin();itr!=(c).end();++itr)
#define rfitr(c, itr) for(auto itr=(c).rbegin();itr!=(c).rend();++itr)
#define chmax(a, b) if((a)<(b)){(a)=(b);}
#define chmin(a, b) if((a)>(b)){(a)=(b);}
#define nli(i, n) " \n"[(i) == (n)]
#define mecpy(a, b) memcpy(a, b, sizeof(a))
#define meset(a, b) memset(a, b, sizeof(a))
#define reopen(x) { freopen(#x".in", "r", stdin); freopen(#x".out", "w", stdout); }
#define ls(i) ((i) << 1)
#define rs(i) (((i) << 1) | 1)
#define by(x) [](const auto& a, const auto& b) { return a.x < b.x; }
#define fi first
#define se second
#define pb push_back
#define mp make_pair
using namespace std;
typedef long long ll;
typedef vector<int> vi;
mt19937 _rnd(std::chrono::system_clock::now().time_since_epoch().count());
int rnd(int l, int r) { return std::uniform_int_distribution<int>(l, r)(_rnd); }
clock_t startTime;
inline double getCurrentTime() {return (double)(clock() - startTime) / CLOCKS_PER_SEC;}
inline void fastio(){ios::sync_with_stdio(false);cin.tie(nullptr);cout.tie(nullptr);}
constexpr int inf = 0x3f3f3f3f;
constexpr double eps = 1e-8;
/* clang-format on */

constexpr int N = 200005;
vi adj[N];
vector<pair<int, int>> ans;

void dfs(int u, int p) {
	for(int v : adj[u]) {
		if(v == p)
			continue;
		ans.pb({u, v});
		ans.pb({v, u});
		ans.pb({u, u});
		dfs(v, u);
	}
}

void solve() {
	int n;
	cin >> n;

	Fe(i, 1, n) adj[i].clear();
	ans.clear();

	F(i, 0, n - 1) {
		int u, v;
		cin >> u >> v;
		adj[u].pb(v);
		adj[v].pb(u);
	}

	ans.pb({1, 1});

	dfs(1, 0);

	cout << ans.size() << endl;
	for(auto& op : ans) {
		cout << op.fi << " " << op.se << endl;
	}
}

signed main() {
	fastio();
	int t;
	cin >> t;
	while(t--) {
		solve();
	}
	return 0;
}
