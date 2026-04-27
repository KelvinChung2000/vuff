module m;
  int q[$];
  initial begin
    q = {};
    if (q == {}) x = 1;
  end
endmodule
// expected -----
module m;
  int q[$];
  initial begin
    q = {};
    if (q == {}) x = 1;
  end
endmodule
