#include <iostream>
#include <utility>
#include <vector>
static/*_*/std::vector<int>/*_*/o[0606505];/**/static/*_*/std::vector<std::pair<int,/*_*/int>>/**/l;
static/*_*/void/*_*/i(int/**/l0,/**/int/*_*/ll)/*_*/{/*_*/for/**/(int/**/lo/*_*/:/*_*/o[l0])/**/{/*_*/if/*_*/(lo/**/==/*_*/ll)/*_*/continue;/**/l.emplace_back(l0,/**/lo);/**/l.emplace_back(lo,/*_*/l0);/*_*/l.emplace_back(l0,/*_*/l0);/**/i(lo,/*_*/l0);/**/}/*_*/}
static/**/void/*_*/l1()/**/{/*_*/int/*_*/l0;/**/std::cin/**/>>/**/l0;/*_*/for/*_*/(int/**/lo/**/=/*_*/01;/*_*/lo/*_*/<=/**/l0;/*_*/++lo)/*_*/o[lo].clear();/**/l.clear();/**/for/*_*/(int/**/o1/**/=/**/0b0;/*_*/o1/**/</**/l0/**/-/**/0x1;/**/++o1)/**/{/**/int/*_*/ll,/*_*/li;/**/std::cin/**/>>/**/ll/**/>>/**/li;/*_*/o[ll].push_back(li);/*_*/o[li].push_back(ll);/**/}/*_*/l.emplace_back(0x1,/*_*/0x1);/**/i(01,/*_*/0b0);/*_*/std::cout/**/<</**/l.size()/*_*/<</*_*/'\n';/**/for/*_*/(auto/**/&o0/*_*/:/*_*/l)/**/std::cout/**/<</*_*/o0.first/**/<</**/'\040'/*_*/<</**/o0.second/*_*/<</*_*/'\n';/**/}
int/*_*/main()/**/{/*_*/std::ios::sync_with_stdio(false);/*_*/std::cin.tie(nullptr);/**/std::cout.tie(nullptr);/**/int/**/l0;/*_*/std::cin/*_*/>>/**/l0;/*_*/while/**/(l0--)/**/l1();/**/return/*_*/0b0;/*_*/}
