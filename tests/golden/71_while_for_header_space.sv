module m;
  logic a;
  initial begin
    while(z) a = !a;
    for(int i = 0; i < 10; i = i + 1) a = 1;
    repeat(5) a = 0;
  end
endmodule
// expected -----
module m;
  logic a;
  initial begin
    while (z) a = !a;
    for (int i = 0; i < 10; i = i + 1) a = 1;
    repeat (5) a = 0;
  end
endmodule
