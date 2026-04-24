module m;
  typedef struct { int a; int b; } pair_t;
  pair_t p;
  initial p = '{ a: 1, b: 2 };
endmodule
// expected -----
module m;
  typedef struct { int a; int b; } pair_t;
  pair_t p;
  initial p = '{a: 1, b: 2};
endmodule
