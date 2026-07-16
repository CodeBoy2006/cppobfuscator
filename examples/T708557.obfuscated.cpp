







#include <bits/stdc++.h>




typedef/*_*/__int128/*_*/lll;




















using/**/namespace/*_*/std;
typedef/*_*/long/**/long/*_*/ll;
typedef/*_*/vector<int>/**/vi;
mt19937/**/l0(std::chrono::system_clock::now().time_since_epoch().count());
int/*_*/o11o(int/*_*/oo,/*_*/int/**/ol)/**/{/*_*/return/*_*/std::uniform_int_distribution<int>(oo,/**/ol)(l0);/*_*/}
clock_t/*_*/lo;
inline/**/double/*_*/o110()/*_*/{return/**/(double)(clock()/**/-/**/lo)/*_*// CLOCKS_PER_SEC;}
inline/*_*/void/*_*/o11l(){([&](){ios::sync_with_stdio(false);cin.tie(nullptr);}());([&](){cout.tie(nullptr);}());}
constexpr/**/int/*_*/o1/**/=/*_*/(static_cast<int>(((013403250260ULL xor 014314467617ULL) bitand 0b1011101011001110011011111011111ULL) bitor ((0x805b6777ULL xor 0xbf645848ULL) bitand 0xa298c820ULL)));
constexpr/**/double/*_*/o0/**/=/*_*/1e-8;


constexpr/*_*/int/**/li/**/=/**/(static_cast<int>(((((0x28ffaa55ULL/*_*/*/*_*/0x3399c0edULL) bitand 0xffffffffULL)/**/+/**/0x5b429029ULL) bitand 0xffffffffULL) xor 0xcb830d9fULL));
vi/*_*/l1[li];
vector<pair<int,/*_*/int>>/*_*/o;

void/**/o111(int/*_*/oo,/*_*/int/**/oi)/**/{
for(int/*_*/ol/**/:/**/l1[oo])/**/{
if(ol/*_*/==/**/oi)
continue;
([&]()->void{o./**/push_back/**/({oo,/**/ol});
o./**/push_back/**/({ol,/**/oo});
o./*_*/push_back/*_*/({oo,/**/oo});}());
([&]()->void{o111(ol,/**/oo);}());
}
}

void/**/o11i()/**/{
int/**/oo;
([&]()->void{cin/*_*/>>/*_*/oo;}());

for(int/**/ol/*_*/=/*_*/(static_cast<int>((((((((0xa810e78eULL/**/>>/*_*/012) bitor (0xa810e78eULL/*_*/<</**/0b10110)) bitand 0xffffffffULL)/**/*/*_*/0xa0acd785ULL) bitand 0xffffffffULL)/*_*/+/**/0xf0ea47e1ULL) bitand 0xffffffffULL) xor 0b1111000000101100101100001111111ULL))/*_*/;/**/ol/*_*/<=/*_*/oo/**/;++/**/ol/*_*/)/**/l1[ol].clear();
([&]()->void{o.clear();}());

for(int/**/i0/*_*/=/**/00/*_*/;/*_*/i0/**/</*_*/oo/**/-/*_*/(static_cast<int>(((013203400273ULL xor 0x5a0e00baULL) bitand 014665027621ULL) bitor ((0xccfa2b5cULL xor 0xccfa2b5dULL) bitand 0x992bd06eULL)))/*_*/;++/**/i0/**/)/**/{
int/*_*/oi,/*_*/i1;
([&]()->void{cin/**/>>/*_*/oi/**/>>/**/i1;
l1[oi]./**/push_back/*_*/(i1);}());
([&](){l1[i1]./*_*/push_back/*_*/(oi);}());
}

([&](){o./**/push_back/**/({(static_cast<int>((((((((0xd694659aULL/**/>>/**/0b11010) bitor (0xd694659aULL/*_*/<</**/06)) bitand 0xffffffffULL)/*_*/*/**/015271724063ULL) bitand 0xffffffffULL)/**/+/*_*/015660437050ULL) bitand 0xffffffffULL) xor 0x9d8b7c36ULL)),/*_*/(static_cast<int>(((((0xa1b0f070ULL/**/*/**/0xc6a45639ULL) bitand 0xffffffffULL)/**/+/*_*/0xde9eac4aULL) bitand 0xffffffffULL) xor 0xf789d53bULL))});

o111((static_cast<int>(((0xef04b00bULL xor 0xef04b00aULL) bitand 0xcdb102eaULL) bitor ((0xb883978bULL xor 0xb883978aULL) bitand 0x324efd15ULL))),/*_*/0b0);}());

([&](){cout/**/<</**/o.size()/*_*/<</**/'\n'/*_*/;}());
for(auto&/**/io/*_*/:/*_*/o)/*_*/{
([&]()->void{cout/**/<</*_*/io./*_*/first/*_*/<</**/"\040"/**/<</**/io./**/second/*_*/<</*_*/'\n'/**/;}());
}
}

signed/*_*/main()/*_*/{
([&](){o11l();}());
int/**/oo;
([&]()->void{cin/*_*/>>/*_*/oo;}());
while(oo--)/*_*/{
([&](){o11i();}());
}
return/**/0b0;
}
